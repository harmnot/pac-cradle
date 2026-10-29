//! Background jobs (the `jobs` table, migration `…_jobs.sql`).

use std::fmt;
use std::time::Duration;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use uuid::Uuid;

use crate::error::{AppError, ErrorCode};

/// Attempts a job gets unless its kind says otherwise.
pub const DEFAULT_MAX_ATTEMPTS: i32 = 5;

/// A kind of job: its name in `jobs.kind` and the payload it carries. Services add jobs by
/// kind ([`NewJob::of`]); a handler in `jobs/` runs them.
pub trait JobKind {
    /// The name stored in `jobs.kind`: lower-case snake_case, at most 64 characters. Never
    /// rename one that may still be queued.
    const NAME: &'static str;
    /// Attempts before the job is dead-lettered.
    const MAX_ATTEMPTS: i32 = DEFAULT_MAX_ATTEMPTS;
    /// What the job needs to run, stored as JSON in `jobs.payload`.
    type Payload: Serialize + DeserializeOwned + Send + 'static;
}

/// A job to add to the queue.
#[derive(Debug, Clone)]
pub struct NewJob {
    /// The id it will have.
    pub id: Uuid,
    /// [`JobKind::NAME`].
    pub kind: &'static str,
    /// The serialized [`JobKind::Payload`].
    pub payload: Value,
    /// [`JobKind::MAX_ATTEMPTS`].
    pub max_attempts: i32,
    /// How long to wait before it may run; zero runs it as soon as a worker is free.
    pub delay: Duration,
}

impl NewJob {
    /// A job of kind `K` with `payload`, due now.
    pub fn of<K: JobKind>(payload: &K::Payload) -> Result<Self, AppError> {
        let payload = serde_json::to_value(payload)
            .map_err(|error| AppError::new(ErrorCode::Internal).with_source(error))?;
        Ok(Self {
            id: crate::shared::id::new_id(),
            kind: K::NAME,
            payload,
            max_attempts: K::MAX_ATTEMPTS,
            delay: Duration::ZERO,
        })
    }

    /// The same job, due after `delay`.
    pub fn after(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }
}

/// A job a worker has claimed. Its lease runs until the worker reports how it went.
#[derive(Debug, Clone)]
pub struct Job {
    /// The job's id.
    pub id: Uuid,
    /// [`JobKind::NAME`].
    pub kind: String,
    /// The payload as stored.
    pub payload: Value,
    /// Claims so far, this one included (1 on the first run).
    pub attempts: i32,
    /// Claims allowed before the job is dead.
    pub max_attempts: i32,
}

/// Why a job didn't finish.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobError {
    /// Something that may work later (a provider down, a timeout): retried with backoff
    /// until the job is out of attempts.
    Retry(String),
    /// Something that will never work (a payload that can't be read, a row that is gone):
    /// dead-lettered now.
    GiveUp(String),
}

impl JobError {
    /// The reason, as stored in `jobs.last_error`.
    pub fn reason(&self) -> &str {
        match self {
            Self::Retry(reason) | Self::GiveUp(reason) => reason,
        }
    }
}

impl fmt::Display for JobError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.reason())
    }
}

/// What housekeeping did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Tidied {
    /// Jobs whose worker stopped mid-run, back in the queue.
    pub requeued: u64,
    /// Jobs whose worker stopped mid-run on their last attempt, now dead.
    pub buried: u64,
    /// Done jobs past their retention, deleted.
    pub deleted: u64,
}

/// Where a job went after a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobOutcome {
    /// It finished.
    Done,
    /// It failed and runs again after this long.
    Retrying(Duration),
    /// It failed for good (out of attempts, or given up) and waits in the dead letters.
    Dead,
    /// Its lease ran out before the run ended, so the job had already moved on (back in the
    /// queue or to another worker) and this run's result was dropped.
    Lost,
}
