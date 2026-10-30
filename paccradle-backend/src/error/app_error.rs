use std::error::Error as StdError;

use axum::response::{IntoResponse, Response};
use serde_json::{Map, Value};

use super::ErrorCode;

/// An error to answer a request with: a code, numbers or names for its message (`meta`),
/// and the error that caused it, which is logged and never sent.
#[derive(Debug, thiserror::Error)]
#[error("{code}")]
pub struct AppError {
    code: ErrorCode,
    meta: Map<String, Value>,
    #[source]
    source: Option<Box<dyn StdError + Send + Sync>>,
}

/// What an error response says, kept in the response's extensions until the `error_body`
/// middleware writes it out in the caller's language.
#[derive(Debug, Clone)]
pub struct ErrorReport {
    /// The error code.
    pub code: ErrorCode,
    /// Values for the message's placeholders, also sent to the caller as `meta`.
    pub meta: Map<String, Value>,
}

impl AppError {
    /// An error with `code` and no meta.
    pub fn new(code: ErrorCode) -> Self {
        Self {
            code,
            meta: Map::new(),
            source: None,
        }
    }

    /// Adds a value for the message and the response's `meta`, e.g. `("field", "height_mm")`.
    pub fn with_meta(mut self, key: &str, value: impl Into<Value>) -> Self {
        self.meta.insert(key.to_owned(), value.into());
        self
    }

    /// Keeps the error that caused this one, for the logs.
    pub fn with_source(mut self, source: impl StdError + Send + Sync + 'static) -> Self {
        self.source = Some(Box::new(source));
        self
    }

    /// The error code.
    pub fn code(&self) -> ErrorCode {
        self.code
    }

    /// The values for its message (stored with a capture whose run failed, M1-D5).
    pub fn meta(&self) -> &Map<String, Value> {
        &self.meta
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let source = self.source.as_ref().map(ToString::to_string);
        if self.code.status().is_server_error() {
            tracing::error!(code = %self.code, source, "request failed");
        } else {
            tracing::debug!(code = %self.code, source, "request refused");
        }
        let mut response = self.code.status().into_response();
        response.extensions_mut().insert(ErrorReport {
            code: self.code,
            meta: self.meta,
        });
        response
    }
}

/// Database failures. One that means the database can't be reached answers 503
/// `SERVICE_UNAVAILABLE` (`meta.dependency = "database"`); anything else is a bug or a broken
/// rule and answers 500. The driver's text stays in the logs.
impl From<sqlx::Error> for AppError {
    fn from(error: sqlx::Error) -> Self {
        let unreachable = matches!(
            error,
            sqlx::Error::PoolTimedOut
                | sqlx::Error::PoolClosed
                | sqlx::Error::Io(_)
                | sqlx::Error::Tls(_)
        );
        if unreachable {
            return Self::new(ErrorCode::ServiceUnavailable)
                .with_meta("dependency", "database")
                .with_source(error);
        }
        Self::new(ErrorCode::Internal).with_source(error)
    }
}
