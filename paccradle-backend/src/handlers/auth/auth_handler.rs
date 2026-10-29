use axum::extract::State;
use axum::http::header::{CACHE_CONTROL, SET_COOKIE};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::{Extension, Json};

use super::refresh_cookie;
use crate::AppState;
use crate::error::{AppError, ErrorCode};
use crate::models::session::SignedIn;
use crate::schemas::identity::SessionResponse;
use crate::schemas::passkey::{PasskeyOptionsResponse, PasskeySignInRequest};
use crate::service::passkey::PasskeySignIn;
use crate::shared::api_surface::ApiSurface;

/// The answer to a sign-in or refresh: the access token in JSON, the refresh token in its
/// cookie. Marked `no-store`, and never compressed (it sets a cookie, or is `no-store`).
pub fn session_response(state: &AppState, (signed_in, status): (SignedIn, StatusCode)) -> Response {
    let surface = signed_in.surface;
    let secure = refresh_cookie::secure(&state.config, surface);
    let cookie = signed_in
        .refresh
        .as_ref()
        .map(|(token, lasts)| refresh_cookie::set(surface, (token, *lasts), secure));
    let mut response = (status, Json(SessionResponse::from(signed_in))).into_response();
    let headers = response.headers_mut();
    if let Some(cookie) = cookie {
        headers.insert(SET_COOKIE, cookie);
    }
    headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

/// `POST /auth/refresh`: a new access token (and refresh cookie) from the refresh cookie.
/// `AUTH_SESSION_EXPIRED` clears the cookie: the user signs in again.
pub async fn refresh(
    Extension(surface): Extension<ApiSurface>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Response {
    let secure = refresh_cookie::secure(&state.config, surface);
    let Some(token) = refresh_cookie::read(&headers, surface) else {
        return session_ended(surface, secure);
    };
    match state.sessions.refresh((surface, &token)).await {
        Ok(signed_in) => session_response(&state, (signed_in, StatusCode::OK)),
        Err(error) if error.code() == ErrorCode::AuthSessionExpired => {
            session_ended(surface, secure)
        }
        Err(error) => error.into_response(),
    }
}

/// `POST /auth/sign-out`: ends the cookie's session and clears the cookie. Always 204.
pub async fn sign_out(
    Extension(surface): Extension<ApiSurface>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    if let Some(token) = refresh_cookie::read(&headers, surface) {
        state.sessions.sign_out((surface, &token)).await?;
    }
    let secure = refresh_cookie::secure(&state.config, surface);
    let mut response = StatusCode::NO_CONTENT.into_response();
    response
        .headers_mut()
        .insert(SET_COOKIE, refresh_cookie::clear(surface, secure));
    Ok(response)
}

/// `POST /auth/passkey/sign-in/options`: a ceremony for `navigator.credentials.get()`.
pub async fn passkey_options(
    Extension(surface): Extension<ApiSurface>,
    State(state): State<AppState>,
) -> Result<Json<PasskeyOptionsResponse>, AppError> {
    let options = state.passkeys.sign_in_options(surface).await?;
    Ok(Json(PasskeyOptionsResponse::from(options)))
}

/// `POST /auth/passkey/sign-in`: signs in with the browser's answer.
pub async fn passkey_sign_in(
    Extension(surface): Extension<ApiSurface>,
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<PasskeySignInRequest>,
) -> Result<Response, AppError> {
    let replacing = refresh_cookie::read(&headers, surface);
    let sign_in = PasskeySignIn {
        surface,
        ceremony_id: body.ceremony_id,
        credential: body.credential.decode()?,
        replacing: replacing.as_deref(),
    };
    let signed_in = state.passkeys.sign_in(sign_in).await?;
    Ok(session_response(&state, (signed_in, StatusCode::OK)))
}

/// `AUTH_SESSION_EXPIRED`, clearing the cookie.
fn session_ended(surface: ApiSurface, secure: bool) -> Response {
    let mut response = AppError::new(ErrorCode::AuthSessionExpired).into_response();
    response
        .headers_mut()
        .insert(SET_COOKIE, refresh_cookie::clear(surface, secure));
    response
}
