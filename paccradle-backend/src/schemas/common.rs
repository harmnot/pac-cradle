use serde::Serialize;
use serde_json::{Map, Value};

/// Every error response: `{ "error": { "code", "message", "meta" } }`.
#[derive(Debug, Serialize)]
pub struct ErrorBody {
    /// The error.
    pub error: ErrorDetail,
}

/// One error.
#[derive(Debug, Serialize)]
pub struct ErrorDetail {
    /// Stable code, e.g. `RATE_LIMITED`.
    pub code: &'static str,
    /// In the caller's language (`Accept-Language`), for showing as is.
    pub message: String,
    /// The values in the message, e.g. `{ "retry_after_seconds": 12 }`. Often empty.
    pub meta: Map<String, Value>,
}
