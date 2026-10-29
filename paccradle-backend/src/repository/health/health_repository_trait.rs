use crate::error::AppError;
use crate::shared::future::BoxFuture;

/// Checks on the database itself.
pub trait HealthRepository: Send + Sync {
    /// Runs the smallest query there is.
    fn ping(&self) -> BoxFuture<'_, Result<(), AppError>>;
}
