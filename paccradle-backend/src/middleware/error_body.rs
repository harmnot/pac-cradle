//! Error bodies: every 4xx and 5xx response leaves the API as
//! `{ "error": { "code", "message", "meta" } }`, with the message in the caller's language.
//! An [`AppError`](crate::error::AppError) brings its code and meta; a response made by
//! framework code (a rejected extractor, a caught panic, a 405) gets the code closest to its
//! status. Raw framework text never reaches the caller.

use axum::body::Body;
use axum::extract::Request;
use axum::http::HeaderValue;
use axum::http::header::{CONTENT_ENCODING, CONTENT_LENGTH, CONTENT_TYPE};
use axum::middleware::Next;
use axum::response::Response;
use serde_json::Map;

use crate::error::{ErrorCode, ErrorReport};
use crate::locales::Locale;
use crate::schemas::common::{ErrorBody, ErrorDetail};

/// Sent if the error body can't be serialized, which a map of JSON values never fails at.
const FALLBACK_BODY: &str = r#"{"error":{"code":"INTERNAL_ERROR","message":"Something went wrong on our side. Please try again.","meta":{}}}"#;

/// Picks the caller's language (also left in the request's extensions as a [`Locale`] for the
/// handlers), runs the request, and writes the error body if the response is an error.
pub async fn render(mut request: Request, next: Next) -> Response {
    let locale = Locale::from_headers(request.headers());
    request.extensions_mut().insert(locale);
    let response = next.run(request).await;
    let status = response.status();
    if !status.is_client_error() && !status.is_server_error() {
        return response;
    }

    let (mut parts, _framework_body) = response.into_parts();
    let report = parts
        .extensions
        .remove::<ErrorReport>()
        .unwrap_or_else(|| ErrorReport {
            code: ErrorCode::from_status(status),
            meta: Map::new(),
        });
    let body = ErrorBody {
        error: ErrorDetail {
            code: report.code.as_str(),
            message: locale.error_message(&report),
            meta: report.meta,
        },
    };
    let bytes = serde_json::to_vec(&body).unwrap_or_else(|_| FALLBACK_BODY.as_bytes().to_vec());
    parts.headers.remove(CONTENT_LENGTH);
    parts.headers.remove(CONTENT_ENCODING);
    parts
        .headers
        .insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    Response::from_parts(parts, Body::from(bytes))
}
