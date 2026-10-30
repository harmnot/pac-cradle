use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;

use serde_json::Value;

use crate::models::job::{Job, JobError, JobKind};
use crate::shared::future::BoxFuture;

/// Runs the jobs of one kind, with their payload already read.
pub trait JobHandler: Send + Sync + 'static {
    /// The kind it runs.
    type Kind: JobKind;

    /// Runs one job. Return [`JobError::Retry`] for failures that may pass, and
    /// [`JobError::GiveUp`] for ones that never will.
    fn run(
        &self,
        payload: <Self::Kind as JobKind>::Payload,
    ) -> impl Future<Output = Result<(), JobError>> + Send;

    /// Called once a job has failed for good: it gave up, or its last attempt failed. A
    /// handler whose job leaves a row waiting (a capture's vision run) records that here, so
    /// the row doesn't wait forever. Does nothing unless a handler says otherwise.
    fn failed_for_good(
        &self,
        payload: <Self::Kind as JobKind>::Payload,
    ) -> impl Future<Output = ()> + Send {
        drop(payload);
        async {}
    }
}

/// A handler of any kind, taking its payload as stored.
trait StoredJobHandler: Send + Sync {
    fn run_stored(&self, payload: Value) -> BoxFuture<'_, Result<(), JobError>>;

    fn failed_stored(&self, payload: Value) -> BoxFuture<'_, ()>;
}

impl<H: JobHandler> StoredJobHandler for H {
    fn run_stored(&self, payload: Value) -> BoxFuture<'_, Result<(), JobError>> {
        Box::pin(async move {
            let payload = serde_json::from_value(payload)
                .map_err(|error| JobError::GiveUp(format!("the payload can't be read: {error}")))?;
            JobHandler::run(self, payload).await
        })
    }

    fn failed_stored(&self, payload: Value) -> BoxFuture<'_, ()> {
        Box::pin(async move {
            // A payload that can't be read never ran, so there is nothing to record.
            if let Ok(payload) = serde_json::from_value(payload) {
                JobHandler::failed_for_good(self, payload).await;
            }
        })
    }
}

/// The handlers a worker runs, one per kind.
#[derive(Clone, Default)]
pub struct JobHandlers {
    by_kind: HashMap<&'static str, Arc<dyn StoredJobHandler>>,
}

impl JobHandlers {
    /// No handlers.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds `handler` for its kind, replacing an earlier one of the same kind.
    pub fn with<H: JobHandler>(mut self, handler: H) -> Self {
        self.by_kind.insert(H::Kind::NAME, Arc::new(handler));
        self
    }

    /// The kinds there is a handler for, sorted. The worker claims only these.
    pub fn kinds(&self) -> Vec<&'static str> {
        let mut kinds: Vec<_> = self.by_kind.keys().copied().collect();
        kinds.sort_unstable();
        kinds
    }

    /// Runs `job` with its kind's handler. When the run fails for good (it gave up, or it was
    /// the last attempt), the handler hears of it before the result is returned.
    pub async fn run(&self, job: &Job) -> Result<(), JobError> {
        let Some(handler) = self.by_kind.get(job.kind.as_str()) else {
            return Err(JobError::GiveUp(format!(
                "no handler for kind {}",
                job.kind
            )));
        };
        let result = handler.run_stored(job.payload.clone()).await;
        let for_good = match &result {
            Ok(()) => false,
            Err(JobError::GiveUp(_)) => true,
            Err(JobError::Retry(_)) => job.attempts >= job.max_attempts,
        };
        if for_good {
            handler.failed_stored(job.payload.clone()).await;
        }
        result
    }
}
