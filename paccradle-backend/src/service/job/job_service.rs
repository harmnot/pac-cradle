use std::sync::Arc;
use std::time::Duration;

use sqlx::PgConnection;

use super::JobService;
use crate::error::AppError;
use crate::models::job::{Job, JobError, JobOutcome, NewJob, Tidied};
use crate::repository::job::JobRepository;
use crate::shared::future::BoxFuture;

/// How long a claim holds a job. Every job kind must finish well within it (vision and
/// exports take seconds); a worker that dies mid-job frees it after this long.
pub const LEASE: Duration = Duration::from_secs(5 * 60);

/// How long done jobs are kept, for looking into what happened.
pub const DONE_RETENTION: Duration = Duration::from_secs(7 * 24 * 60 * 60);

/// `last_error` holds at most this many characters.
const MAX_ERROR_CHARS: usize = 2000;

/// How long a failed job waits before its next attempt: `first` after the first failure,
/// doubling each time, never more than `max`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicy {
    /// The wait after the first failed attempt.
    pub first: Duration,
    /// The longest wait.
    pub max: Duration,
}

impl Default for RetryPolicy {
    /// 10 s, 20 s, 40 s, 80 s… up to an hour. With the default 5 attempts, a job is retried
    /// for about 2.5 minutes before it is dead.
    fn default() -> Self {
        Self {
            first: Duration::from_secs(10),
            max: Duration::from_secs(60 * 60),
        }
    }
}

impl RetryPolicy {
    /// The wait after failed attempt number `attempt` (1 for the first).
    pub fn delay(&self, attempt: i32) -> Duration {
        let doublings = u32::try_from(attempt.saturating_sub(1))
            .unwrap_or_default()
            .min(30);
        self.first.saturating_mul(1 << doublings).min(self.max)
    }
}

/// [`JobService`] on a [`JobRepository`].
pub struct DefaultJobService {
    jobs: Arc<dyn JobRepository>,
    retry: RetryPolicy,
}

impl DefaultJobService {
    /// A queue on `jobs` with the default [`RetryPolicy`].
    pub fn new(jobs: Arc<dyn JobRepository>) -> Self {
        Self {
            jobs,
            retry: RetryPolicy::default(),
        }
    }

    /// The same queue with another retry policy.
    pub fn with_retry(mut self, retry: RetryPolicy) -> Self {
        self.retry = retry;
        self
    }
}

impl JobService for DefaultJobService {
    fn enqueue<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        job: &'a NewJob,
    ) -> BoxFuture<'a, Result<(), AppError>> {
        self.jobs.insert(conn, job)
    }

    fn claim<'a>(
        &'a self,
        kinds: &'a [&'static str],
    ) -> BoxFuture<'a, Result<Option<Job>, AppError>> {
        Box::pin(async move {
            if kinds.is_empty() {
                return Ok(None);
            }
            self.jobs.claim(kinds, LEASE).await
        })
    }

    fn finish<'a>(
        &'a self,
        job: &'a Job,
        result: Result<(), JobError>,
    ) -> BoxFuture<'a, Result<JobOutcome, AppError>> {
        Box::pin(async move {
            let (applied, outcome) = match result {
                Ok(()) => (self.jobs.mark_done(job).await?, JobOutcome::Done),
                Err(JobError::Retry(reason)) if job.attempts < job.max_attempts => {
                    let delay = self.retry.delay(job.attempts);
                    let applied = self.jobs.requeue(job, (delay, &shorten(&reason))).await?;
                    (applied, JobOutcome::Retrying(delay))
                }
                Err(error) => {
                    let reason = shorten(error.reason());
                    (self.jobs.mark_dead(job, &reason).await?, JobOutcome::Dead)
                }
            };
            Ok(if applied { outcome } else { JobOutcome::Lost })
        })
    }

    fn tidy(&self) -> BoxFuture<'_, Result<Tidied, AppError>> {
        Box::pin(async move {
            let (requeued, buried) = self.jobs.release_expired().await?;
            let deleted = self.jobs.delete_done_older_than(DONE_RETENTION).await?;
            Ok(Tidied {
                requeued,
                buried,
                deleted,
            })
        })
    }
}

/// `reason`, cut to [`MAX_ERROR_CHARS`] characters.
fn shorten(reason: &str) -> String {
    reason.chars().take(MAX_ERROR_CHARS).collect()
}
