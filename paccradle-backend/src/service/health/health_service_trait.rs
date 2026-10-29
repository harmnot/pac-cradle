use crate::error::AppError;
use crate::shared::future::BoxFuture;

/// Readiness of what the API depends on.
pub trait HealthService: Send + Sync {
    /// `Ok` when every dependency answers in time; otherwise `SERVICE_UNAVAILABLE` naming the
    /// first one that didn't (`meta.dependency`).
    fn readiness(&self) -> BoxFuture<'_, Result<(), AppError>>;
}
