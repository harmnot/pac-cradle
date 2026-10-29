//! The span every request's log lines sit in. It holds no query string, header or body, so
//! tokens and personal data never reach the logs.

use axum::extract::Request;
use tower_http::LatencyUnit;
use tower_http::trace::DefaultOnResponse;
use tracing::{Level, Span};

use super::request_id::REQUEST_ID_HEADER;

/// `http{method path request_id}`.
pub fn make_span(request: &Request) -> Span {
    let request_id = request
        .headers()
        .get(REQUEST_ID_HEADER)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    tracing::info_span!(
        "http",
        method = %request.method(),
        path = request.uri().path(),
        request_id,
    )
}

/// One `finished processing request` line per request, with its status and latency in ms.
/// Failures are logged where they happen ([`crate::error::AppError`]), not here again.
pub fn on_response() -> DefaultOnResponse {
    DefaultOnResponse::new()
        .level(Level::INFO)
        .latency_unit(LatencyUnit::Millis)
}
