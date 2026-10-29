use uuid::Uuid;

use crate::error::AppError;
use crate::models::audit::AdminWrite;
use crate::shared::future::BoxFuture;

/// Records admin writes. A write that can't be recorded must not run.
pub trait AuditService: Send + Sync {
    /// Records a write about to run.
    fn start<'a>(&'a self, write: &'a AdminWrite) -> BoxFuture<'a, Result<(), AppError>>;

    /// Records how it ended (the response status).
    fn finish(&self, entry: (Uuid, u16)) -> BoxFuture<'_, Result<(), AppError>>;
}
