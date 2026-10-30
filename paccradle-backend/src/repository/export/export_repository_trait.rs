use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::export::{Export, ExportFile, ExportListQuery, ExportSummary, NewExport};
use crate::shared::future::BoxFuture;

/// Reads and writes `exports`. Every method runs on the connection it is given.
pub trait ExportRepository: Send + Sync {
    /// Adds a pending export.
    fn insert<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        export: &'a NewExport,
    ) -> BoxFuture<'a, Result<(), AppError>>;

    /// The export with `id`, with its design's owner.
    fn find<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        id: Uuid,
    ) -> BoxFuture<'a, Result<Option<Export>, AppError>>;

    /// Records a pending export's files: done. `false` when it isn't pending any more.
    fn finish<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        done: (Uuid, &'a [ExportFile]),
    ) -> BoxFuture<'a, Result<bool, AppError>>;

    /// Records why a pending export failed. `false` when it isn't pending any more.
    fn fail<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        failed: (Uuid, &'a str),
    ) -> BoxFuture<'a, Result<bool, AppError>>;

    /// One page of a design's exports.
    fn list<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        query: &'a ExportListQuery,
    ) -> BoxFuture<'a, Result<Vec<ExportSummary>, AppError>>;

    /// How many of a design's exports pass the query's filter.
    fn count<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        query: &'a ExportListQuery,
    ) -> BoxFuture<'a, Result<u64, AppError>>;
}
