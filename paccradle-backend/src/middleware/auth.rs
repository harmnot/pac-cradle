//! Who a request comes from: the `Authorization: Bearer <access token>` header, checked for the
//! surface the route belongs to (each surface router puts its [`ApiSurface`] in the request's
//! extensions). A client token is refused on the admin surface and the other way round.

use axum::extract::FromRequestParts;
use axum::http::HeaderMap;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;

use crate::AppState;
use crate::error::{AppError, ErrorCode};
use crate::models::session::AuthUser;
use crate::shared::api_surface::ApiSurface;

/// The token in an `Authorization: Bearer …` header.
pub fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    let value = headers.get(AUTHORIZATION)?.to_str().ok()?;
    let (scheme, token) = value.split_once(' ')?;
    let token = token.trim();
    (scheme.eq_ignore_ascii_case("bearer") && !token.is_empty()).then_some(token)
}

/// Handlers take an [`AuthUser`] to require a signed-in caller (a guest included; services
/// refuse guests where an account is needed). `AUTH_REQUIRED` without a valid access token:
/// the web app then refreshes it and tries again.
impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, AppError> {
        // The admin guard has already checked it.
        if let Some(user) = parts.extensions.get::<AuthUser>() {
            return Ok(*user);
        }
        let surface = *parts
            .extensions
            .get::<ApiSurface>()
            .ok_or_else(|| AppError::new(ErrorCode::Internal))?;
        let token =
            bearer_token(&parts.headers).ok_or_else(|| AppError::new(ErrorCode::AuthRequired))?;
        state.sessions.authenticate((surface, token))
    }
}
