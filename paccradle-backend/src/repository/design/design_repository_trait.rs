use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::design::{DesignListQuery, DesignSummary, NewDesign, SavedDesign};
use crate::shared::future::BoxFuture;

/// Reads and writes `designs`. Every method runs on the connection it is given, so a service
/// can put several in one transaction.
pub trait DesignRepository: Send + Sync {
    /// Adds a design at version 1.
    fn insert<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        design: &'a NewDesign,
    ) -> BoxFuture<'a, Result<SavedDesign, AppError>>;

    /// The design with `id`.
    fn find<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        id: Uuid,
    ) -> BoxFuture<'a, Result<Option<SavedDesign>, AppError>>;

    /// The design with `id`, locked until the transaction ends (`FOR UPDATE`).
    fn lock<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        id: Uuid,
    ) -> BoxFuture<'a, Result<Option<SavedDesign>, AppError>>;

    /// Writes the design's name, spec and version; returns it as stored.
    fn update<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        design: &'a SavedDesign,
    ) -> BoxFuture<'a, Result<SavedDesign, AppError>>;

    /// One page of a user's designs, filtered and sorted.
    fn list<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        query: &'a DesignListQuery,
    ) -> BoxFuture<'a, Result<Vec<DesignSummary>, AppError>>;

    /// How many of a user's designs pass the query's filters.
    fn count<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        query: &'a DesignListQuery,
    ) -> BoxFuture<'a, Result<u64, AppError>>;
}
