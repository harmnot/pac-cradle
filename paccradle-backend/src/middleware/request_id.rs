//! Request IDs: a UUID v7 per request, made here and never taken from the caller (Nginx
//! clears the header too). The web app shows it in error messages, and every log line of the
//! request carries it.

use axum::extract::Request;
use axum::http::{HeaderName, HeaderValue};
use tower_http::request_id::{MakeRequestId, RequestId};

use crate::shared::id::new_id;

/// The request ID header.
pub const REQUEST_ID_HEADER: HeaderName = HeaderName::from_static("x-request-id");

/// Drops an `x-request-id` sent by the caller, so the API's own ID is always used.
pub async fn drop_client_request_id(mut request: Request) -> Request {
    request.headers_mut().remove(REQUEST_ID_HEADER);
    request
}

/// Makes UUID v7 request IDs.
#[derive(Debug, Clone, Copy, Default)]
pub struct MakeRequestUuidV7;

impl MakeRequestId for MakeRequestUuidV7 {
    fn make_request_id<B>(&mut self, _request: &axum::http::Request<B>) -> Option<RequestId> {
        HeaderValue::from_str(&new_id().to_string())
            .ok()
            .map(RequestId::new)
    }
}
