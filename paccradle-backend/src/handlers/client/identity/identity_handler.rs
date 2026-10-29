use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use axum::{Extension, Json};

use crate::AppState;
use crate::error::AppError;
use crate::handlers::auth::auth_handler::session_response;
use crate::handlers::auth::refresh_cookie;
use crate::locales::Locale;
use crate::models::session::AuthUser;
use crate::schemas::identity::{
    AccountResponse, EmailLinkRequest, EmailLinkSignInRequest, PasswordSignInRequest,
    SetPasswordRequest,
};
use crate::schemas::passkey::{
    PasskeyOptionsResponse, PasskeyRegistrationRequest, PasskeyResponse,
};
use crate::service::identity::{self, PasswordSignIn, SetPassword};
use crate::service::passkey::PasskeyRegistration;
use crate::shared::api_surface::ApiSurface;

/// `POST /auth/guest`: a guest session, so a visitor can start before signing up. 201.
pub async fn start_guest(
    State(state): State<AppState>,
    Extension(locale): Extension<Locale>,
) -> Result<Response, AppError> {
    let signed_in = state.identity.start_guest(locale).await?;
    Ok(session_response(&state, (signed_in, StatusCode::CREATED)))
}

/// `POST /auth/email-link`: emails a link that signs up (confirming the address) or signs in.
/// 202 whether or not the address has an account.
pub async fn request_email_link(
    State(state): State<AppState>,
    Extension(locale): Extension<Locale>,
    Json(body): Json<EmailLinkRequest>,
) -> Result<StatusCode, AppError> {
    let request = identity::EmailLinkRequest {
        email: body.email()?,
        locale,
    };
    state.identity.request_email_link(request).await?;
    Ok(StatusCode::ACCEPTED)
}

/// `POST /auth/email-link/sign-in`: signs in with the link's token.
pub async fn email_link_sign_in(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<EmailLinkSignInRequest>,
) -> Result<Response, AppError> {
    let replacing = refresh_cookie::read(&headers, ApiSurface::Client);
    let signed_in = state
        .identity
        .sign_in_with_email_link((body.token()?, replacing.as_deref()))
        .await?;
    Ok(session_response(&state, (signed_in, StatusCode::OK)))
}

/// `POST /auth/password/sign-in`.
pub async fn password_sign_in(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<PasswordSignInRequest>,
) -> Result<Response, AppError> {
    let (email, password) = body.parts()?;
    let replacing = refresh_cookie::read(&headers, ApiSurface::Client);
    let sign_in = PasswordSignIn {
        email,
        password,
        replacing: replacing.as_deref(),
    };
    let signed_in = state.identity.sign_in_with_password(sign_in).await?;
    Ok(session_response(&state, (signed_in, StatusCode::OK)))
}

/// `GET /me`: the signed-in user (a guest too).
pub async fn me(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<AccountResponse>, AppError> {
    let account = state.identity.account(&user).await?;
    Ok(Json(AccountResponse::from(account)))
}

/// `PUT /me/password`: sets or changes the password; other sessions end. 204.
pub async fn set_password(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<SetPasswordRequest>,
) -> Result<StatusCode, AppError> {
    let change = SetPassword {
        password: &body.password,
        current_password: body.current_password.as_deref(),
    };
    state.identity.set_password((&user, change)).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /me/passkeys/options`: a ceremony for `navigator.credentials.create()`.
pub async fn passkey_options(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<PasskeyOptionsResponse>, AppError> {
    let options = state.passkeys.registration_options(&user).await?;
    Ok(Json(PasskeyOptionsResponse::from(options)))
}

/// `POST /me/passkeys`: stores the new passkey. 201.
pub async fn add_passkey(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<PasskeyRegistrationRequest>,
) -> Result<(StatusCode, Json<PasskeyResponse>), AppError> {
    let registration = PasskeyRegistration {
        ceremony_id: body.ceremony_id,
        name: body.name()?,
        credential: body.credential.decode()?,
    };
    let passkey = state.passkeys.register((&user, registration)).await?;
    Ok((StatusCode::CREATED, Json(PasskeyResponse::from(passkey))))
}
