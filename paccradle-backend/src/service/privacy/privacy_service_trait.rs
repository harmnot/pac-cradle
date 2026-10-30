use crate::error::AppError;
use crate::shared::future::BoxFuture;

/// What a purge run did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PurgeReport {
    /// Photos deleted from storage and forgotten.
    pub deleted: u64,
    /// More were due than one run takes on: another run was queued to carry on at once.
    pub continued: bool,
}

/// What the app keeps about people, and for how long (CLAUDE.md §8.5).
pub trait PrivacyService: Send + Sync {
    /// Deletes every stored photo past its `purge_after` (30 days after upload), oldest first,
    /// and clears its capture's `storage_key`; outlines and sizes stay. A run stops after a few
    /// minutes and queues the next to carry on. A photo the store can't delete ends the run
    /// with that error (the job retries); those already deleted stay deleted.
    fn purge_due_photos(&self) -> BoxFuture<'_, Result<PurgeReport, AppError>>;
}
