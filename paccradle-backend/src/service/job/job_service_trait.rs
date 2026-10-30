use sqlx::PgConnection;

use crate::error::AppError;
use crate::models::job::{Job, JobError, JobOutcome, NewJob, Schedule, Tidied};
use crate::shared::future::BoxFuture;

/// The job queue, for services (adding jobs) and the worker (running them).
pub trait JobService: Send + Sync {
    /// Adds `job` through `conn`. Pass the caller's open transaction, so the job exists only
    /// if the caller's own writes commit (the outbox rule, `backend-architecture.md` §1).
    fn enqueue<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        job: &'a NewJob,
    ) -> BoxFuture<'a, Result<(), AppError>>;

    /// Adds the next run of `schedule` when it is due: none of its kind queued or running,
    /// and the last finished at least `schedule.every` ago. Says whether it added one.
    fn schedule<'a>(&'a self, schedule: &'a Schedule) -> BoxFuture<'a, Result<bool, AppError>>;

    /// The next due job of one of `kinds`, leased to the caller. `None` when there is none.
    fn claim<'a>(
        &'a self,
        kinds: &'a [&'static str],
    ) -> BoxFuture<'a, Result<Option<Job>, AppError>>;

    /// Records how a claimed job's run ended: done, retried later with backoff, or dead once
    /// out of attempts or given up.
    fn finish<'a>(
        &'a self,
        job: &'a Job,
        result: Result<(), JobError>,
    ) -> BoxFuture<'a, Result<JobOutcome, AppError>>;

    /// Housekeeping: requeues jobs whose worker stopped mid-run, and deletes done jobs past
    /// their retention. Dead jobs are kept.
    fn tidy(&self) -> BoxFuture<'_, Result<Tidied, AppError>>;
}
