//! Cross-site request refusal for the sign-in routes, which read the refresh cookie. Browsers
//! send `Origin` on every POST; one naming another site (or `null`) is refused with `FORBIDDEN`,
//! on top of the cookie's `SameSite=Strict`. Callers that aren't browsers send no `Origin` and
//! carry no cookie jar to abuse, so they pass.

use axum::extract::{Request, State};
use axum::http::HeaderValue;
use axum::http::header::ORIGIN;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::error::{AppError, ErrorCode};

/// Passes requests with no `Origin` header or with `allowed`, the surface's web origin.
pub async fn require(
    State(allowed): State<Option<HeaderValue>>,
    request: Request,
    next: Next,
) -> Response {
    match request.headers().get(ORIGIN) {
        Some(origin) if Some(origin) != allowed.as_ref() => {
            AppError::new(ErrorCode::Forbidden).into_response()
        }
        _ => next.run(request).await,
    }
}
