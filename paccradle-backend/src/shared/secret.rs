//! Values that must never reach a log line or an error message: database URLs with their
//! password, API keys, signing secrets.

use std::fmt;

/// Holds a secret; `{:?}` prints `[redacted]`. Read it with [`Secret::expose`] only where it
/// is handed to the library that needs it.
#[derive(Clone, PartialEq, Eq)]
pub struct Secret<T = String>(T);

impl<T> Secret<T> {
    /// Wraps `value`.
    pub const fn new(value: T) -> Self {
        Self(value)
    }

    /// The secret itself.
    pub const fn expose(&self) -> &T {
        &self.0
    }
}

impl<T> fmt::Debug for Secret<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[redacted]")
    }
}
