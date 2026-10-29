use std::sync::Arc;

use uuid::Uuid;

use super::AuditService;
use crate::error::AppError;
use crate::models::audit::AdminWrite;
use crate::repository::audit::AuditRepository;
use crate::repository::database::Database;
use crate::shared::future::BoxFuture;

/// [`AuditService`] on Postgres.
pub struct DefaultAuditService {
    db: Arc<dyn Database>,
    log: Arc<dyn AuditRepository>,
}

impl DefaultAuditService {
    /// A service appending to `log`.
    pub fn new(db: Arc<dyn Database>, log: Arc<dyn AuditRepository>) -> Self {
        Self { db, log }
    }
}

impl AuditService for DefaultAuditService {
    fn start<'a>(&'a self, write: &'a AdminWrite) -> BoxFuture<'a, Result<(), AppError>> {
        Box::pin(async move {
            let mut conn = self.db.connection().await?;
            self.log.insert(&mut conn, write).await
        })
    }

    fn finish(&self, entry: (Uuid, u16)) -> BoxFuture<'_, Result<(), AppError>> {
        Box::pin(async move {
            let mut conn = self.db.connection().await?;
            self.log.finish(&mut conn, entry).await
        })
    }
}
