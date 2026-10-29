use sqlx::PgPool;

use super::HealthRepository;
use crate::error::AppError;
use crate::shared::future::BoxFuture;

/// [`HealthRepository`] on Postgres.
pub struct PgHealthRepository {
    pool: PgPool,
}

impl PgHealthRepository {
    /// A repository using `pool`.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl HealthRepository for PgHealthRepository {
    fn ping(&self) -> BoxFuture<'_, Result<(), AppError>> {
        Box::pin(async move {
            sqlx::query!("SELECT 1 AS one")
                .fetch_one(&self.pool)
                .await?;
            Ok(())
        })
    }
}
