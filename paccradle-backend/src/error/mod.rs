//! Errors returned to API callers, each with a stable code (`backend-architecture.md` §4.1).
//!
//! Handlers and middleware return [`AppError`]. Its response carries the code and meta in an
//! [`ErrorReport`]; the `error_body` middleware writes the localized JSON body from it.

mod app_error;
mod error_code;

pub use app_error::{AppError, ErrorReport};
pub use error_code::ErrorCode;
