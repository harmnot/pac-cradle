use std::time::Duration;

use sqlx::PgConnection;

use crate::error::AppError;
use crate::models::job::{Job, NewJob};
use crate::shared::future::BoxFuture;

/// Reads and writes `jobs`. The `mark_*` and `requeue` methods change a job only while the
/// claim they are given still holds it (same id, still running, same attempt), and say
/// whether it did: `false` means the lease ran out and the job moved on without this run.
pub trait JobRepository: Send + Sync {
    /// Inserts `job` through `conn`, so it commits or rolls back with the caller's transaction.
    fn insert<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        job: &'a NewJob,
    ) -> BoxFuture<'a, Result<(), AppError>>;

    /// Inserts `job`, due now, unless a job of its kind is queued or running, or finished
    /// (done or dead) less than `every` ago. Says whether it did.
    fn insert_unless_recent<'a>(
        &'a self,
        job: (&'a NewJob, Duration),
    ) -> BoxFuture<'a, Result<bool, AppError>>;

    /// Leases the oldest due queued job of one of `kinds` for `lease`, counting the attempt.
    /// Jobs other workers are claiming at the same moment are skipped, not waited for.
    fn claim<'a>(
        &'a self,
        kinds: &'a [&'static str],
        lease: Duration,
    ) -> BoxFuture<'a, Result<Option<Job>, AppError>>;

    /// Marks the claimed job done.
    fn mark_done<'a>(&'a self, job: &'a Job) -> BoxFuture<'a, Result<bool, AppError>>;

    /// Puts the claimed job back in the queue with `retry`: due after its delay, noting its
    /// error.
    fn requeue<'a>(
        &'a self,
        job: &'a Job,
        retry: (Duration, &'a str),
    ) -> BoxFuture<'a, Result<bool, AppError>>;

    /// Marks the claimed job dead, noting `error`.
    fn mark_dead<'a>(
        &'a self,
        job: &'a Job,
        error: &'a str,
    ) -> BoxFuture<'a, Result<bool, AppError>>;

    /// Running jobs whose lease ran out go back in the queue, or die when out of attempts.
    /// Returns how many of each: `(requeued, dead)`.
    fn release_expired(&self) -> BoxFuture<'_, Result<(u64, u64), AppError>>;

    /// Deletes done jobs that finished more than `age` ago. Returns how many.
    fn delete_done_older_than(&self, age: Duration) -> BoxFuture<'_, Result<u64, AppError>>;
}
