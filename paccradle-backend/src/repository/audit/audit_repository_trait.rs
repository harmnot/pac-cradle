use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::audit::AdminWrite;
use crate::shared::future::BoxFuture;

/// Appends to `admin_audit_log`, on the connection it is given.
pub trait AuditRepository: Send + Sync {
    /// Records an admin write about to run.
    fn insert<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        write: &'a AdminWrite,
    ) -> BoxFuture<'a, Result<(), AppError>>;

    /// Records the write's response status, once.
    fn finish<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        entry: (Uuid, u16),
    ) -> BoxFuture<'a, Result<(), AppError>>;
}
