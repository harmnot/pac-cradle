//! The back office's gate. It wraps every admin path except sign-in, unknown ones included, so
//! callers who aren't admins can't even learn which admin routes exist. It also records every
//! admin write (who, what, when) before it runs; a write that can't be recorded doesn't run.

use axum::extract::{MatchedPath, Request, State};
use axum::http::Method;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use super::auth::bearer_token;
use super::request_id::REQUEST_ID_HEADER;
use crate::AppState;
use crate::error::{AppError, ErrorCode};
use crate::models::audit::AdminWrite;
use crate::models::session::AuthUser;
use crate::models::user::AccountRole;
use crate::shared::api_surface::ApiSurface;
use crate::shared::id::new_id;

/// Lets a request through only with an admin access token (issued by an admin's passkey
/// sign-in on this surface); `AUTH_REQUIRED` otherwise. Writes are logged around the handler.
pub async fn require_admin(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    let admin = match admin_of(&state, &request) {
        Ok(admin) => admin,
        Err(error) => return error.into_response(),
    };
    request.extensions_mut().insert(admin);
    if matches!(
        *request.method(),
        Method::GET | Method::HEAD | Method::OPTIONS
    ) {
        return next.run(request).await;
    }

    let write = AdminWrite {
        id: new_id(),
        admin_id: admin.user_id,
        session_id: admin.session_id,
        request_id: request
            .headers()
            .get(REQUEST_ID_HEADER)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_owned(),
        method: request.method().to_string(),
        route: request
            .extensions()
            .get::<MatchedPath>()
            .map(|route| route.as_str().to_owned()),
        path: format!("{}{}", ApiSurface::Admin.prefix(), request.uri().path()),
    };
    if let Err(error) = state.audit.start(&write).await {
        return error.into_response();
    }
    let response = next.run(request).await;
    if let Err(error) = state
        .audit
        .finish((write.id, response.status().as_u16()))
        .await
    {
        tracing::error!(?error, entry = %write.id, "admin write ran but its status wasn't logged");
    }
    response
}

fn admin_of(state: &AppState, request: &Request) -> Result<AuthUser, AppError> {
    let token =
        bearer_token(request.headers()).ok_or_else(|| AppError::new(ErrorCode::AuthRequired))?;
    let user = state.sessions.authenticate((ApiSurface::Admin, token))?;
    if user.role != AccountRole::Admin {
        return Err(AppError::new(ErrorCode::AuthRequired));
    }
    Ok(user)
}
