use std::sync::Arc;
use std::time::Duration;

use super::HealthService;
use crate::error::{AppError, ErrorCode};
use crate::repository::health::HealthRepository;
use crate::shared::future::BoxFuture;

/// How long the database gets to answer. The deploy polls `/readyz` every 2 s, so a slow
/// answer counts as not ready.
const READY_TIMEOUT: Duration = Duration::from_secs(2);

/// [`HealthService`] checking the database.
pub struct DefaultHealthService {
    database: Arc<dyn HealthRepository>,
}

impl DefaultHealthService {
    /// A service checking `database`.
    pub fn new(database: Arc<dyn HealthRepository>) -> Self {
        Self { database }
    }
}

impl HealthService for DefaultHealthService {
    fn readiness(&self) -> BoxFuture<'_, Result<(), AppError>> {
        Box::pin(async move {
            let unavailable =
                || AppError::new(ErrorCode::ServiceUnavailable).with_meta("dependency", "database");
            match tokio::time::timeout(READY_TIMEOUT, self.database.ping()).await {
                Ok(Ok(())) => Ok(()),
                Ok(Err(error)) => Err(unavailable().with_source(error)),
                Err(_) => Err(unavailable()),
            }
        })
    }
}
