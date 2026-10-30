use std::time::Duration;

use crate::error::AppError;
use crate::shared::future::BoxFuture;

/// An object to store.
pub struct NewObject {
    /// Where: e.g. `photos/<capture id>.jpg`. Letters, digits, `/`, `-`, `_` and `.` only.
    pub key: String,
    /// What.
    pub bytes: Vec<u8>,
    /// Its media type, e.g. `image/jpeg`.
    pub content_type: &'static str,
}

// Photos are people's data: a log line shows the key and size, never the bytes.
impl std::fmt::Debug for NewObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NewObject")
            .field("key", &self.key)
            .field("bytes", &self.bytes.len())
            .field("content_type", &self.content_type)
            .finish()
    }
}

/// A private bucket. A store that can't be reached, or answers with a server error, is
/// `SERVICE_UNAVAILABLE` (`meta.dependency = "storage"`): a job retries, a request says "try
/// again". Anything else it refuses (wrong key, missing bucket) is a setup mistake: 500.
pub trait ObjectStorage: Send + Sync {
    /// Stores `object`, replacing any object with its key.
    fn put(&self, object: NewObject) -> BoxFuture<'_, Result<(), AppError>>;

    /// The object at `key`, or `None` when there is none.
    fn get<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<Option<Vec<u8>>, AppError>>;

    /// Deletes the object at `key`. Deleting one that isn't there succeeds, so a retry is safe.
    fn delete<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<(), AppError>>;

    /// A URL that lets anyone download the object at `key` for `valid_for` (at most 7 days),
    /// and nothing else. Made locally, without a call to the store.
    fn signed_url(&self, request: (&str, Duration)) -> Result<String, AppError>;
}
