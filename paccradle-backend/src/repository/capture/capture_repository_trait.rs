use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::capture::{Capture, DuePhoto, NewCapture};
use crate::shared::future::BoxFuture;

/// Reads and writes `captures`. Every method runs on the connection it is given, so a service
/// can put several in one transaction.
pub trait CaptureRepository: Send + Sync {
    /// Adds a pending capture (run 1) whose photo is kept for `capture.photo_retention`.
    fn insert<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        capture: &'a NewCapture,
    ) -> BoxFuture<'a, Result<Capture, AppError>>;

    /// The capture with `id`.
    fn find<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        id: Uuid,
    ) -> BoxFuture<'a, Result<Option<Capture>, AppError>>;

    /// The capture with `id`, locked until the transaction ends (`FOR UPDATE`): changes to one
    /// capture happen one at a time.
    fn lock<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        id: Uuid,
    ) -> BoxFuture<'a, Result<Option<Capture>, AppError>>;

    /// Writes what may change: status, settings, run, result, confidence, level and error.
    fn save<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        capture: &'a Capture,
    ) -> BoxFuture<'a, Result<(), AppError>>;

    /// Up to `limit` stored photos past their `purge_after`, the longest overdue first.
    fn photos_due<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        limit: i64,
    ) -> BoxFuture<'a, Result<Vec<DuePhoto>, AppError>>;

    /// Notes that `photo` is deleted: its capture's `storage_key` becomes NULL, if it still
    /// names that photo. `false` when it doesn't (another run got there first).
    fn forget_photo<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        photo: &'a DuePhoto,
    ) -> BoxFuture<'a, Result<bool, AppError>>;
}
