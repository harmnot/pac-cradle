//! A time limit on every request. Routes that need longer (photo uploads, M1-D4) get their own.

use std::time::Duration;

use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::error::{AppError, ErrorCode};

/// Answers `REQUEST_TIMEOUT` (503) when the request runs longer than `limit`. The handler's
/// work stops there, so anything that must finish belongs in a job (M1-D2).
pub async fn limit(State(limit): State<Duration>, request: Request, next: Next) -> Response {
    match tokio::time::timeout(limit, next.run(request)).await {
        Ok(response) => response,
        Err(_) => AppError::new(ErrorCode::RequestTimeout)
            .with_meta("timeout_seconds", limit.as_secs())
            .into_response(),
    }
}
