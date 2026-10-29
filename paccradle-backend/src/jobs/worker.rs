use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::watch;
use tokio::task::JoinHandle;
use tracing::Instrument;

use super::JobHandlers;
use crate::models::job::{Job, JobError, JobOutcome, Tidied};
use crate::service::job::JobService;

/// How often an idle worker looks for due jobs.
pub const POLL_INTERVAL: Duration = Duration::from_millis(500);

/// How often housekeeping runs (expired leases, old done jobs).
const TIDY_INTERVAL: Duration = Duration::from_secs(60);

/// Claim loops over the job queue, plus housekeeping.
pub struct Worker {
    jobs: Arc<dyn JobService>,
    handlers: Arc<JobHandlers>,
    poll_interval: Duration,
}

impl Worker {
    /// A worker running `handlers` on the jobs from `jobs`.
    pub fn new(jobs: Arc<dyn JobService>, handlers: JobHandlers) -> Self {
        Self {
            jobs,
            handlers: Arc::new(handlers),
            poll_interval: POLL_INTERVAL,
        }
    }

    /// The same worker, looking for jobs every `interval` when idle.
    pub fn poll_every(mut self, interval: Duration) -> Self {
        self.poll_interval = interval;
        self
    }

    /// Starts `count` claim loops and one housekeeping loop; none when `count` is 0. They stop
    /// once `stop` turns `true`, each finishing the job it is running first.
    pub fn start(self, count: usize, stop: watch::Receiver<bool>) -> Vec<JoinHandle<()>> {
        if count == 0 {
            return Vec::new();
        }
        let worker = Arc::new(self);
        let mut loops = vec![tokio::spawn(Arc::clone(&worker).tidy_loop(stop.clone()))];
        let kinds: Arc<[&'static str]> = worker.handlers.kinds().into();
        if kinds.is_empty() {
            tracing::info!("no job handlers yet, so the workers claim nothing");
            return loops;
        }
        for number in 0..count {
            let claim_loop =
                Arc::clone(&worker).claim_loop((number, Arc::clone(&kinds), stop.clone()));
            loops.push(tokio::spawn(claim_loop));
        }
        loops
    }

    async fn claim_loop(
        self: Arc<Self>,
        (number, kinds, mut stop): (usize, Arc<[&'static str]>, watch::Receiver<bool>),
    ) {
        while !*stop.borrow() {
            match self.jobs.claim(&kinds).await {
                Ok(Some(job)) => self.run(job).await,
                Ok(None) => {
                    if !keep_going(&mut stop, self.poll_interval).await {
                        break;
                    }
                }
                Err(error) => {
                    tracing::warn!(worker = number, ?error, "cannot claim a job");
                    if !keep_going(&mut stop, self.poll_interval).await {
                        break;
                    }
                }
            }
        }
    }

    async fn run(&self, job: Job) {
        let span =
            tracing::info_span!("job", id = %job.id, kind = %job.kind, attempt = job.attempts);
        async {
            let started = Instant::now();
            let handlers = Arc::clone(&self.handlers);
            let claimed = job.clone();
            // In its own task, so a panicking handler fails its job instead of the worker.
            let result = tokio::spawn(async move { handlers.run(&claimed).await })
                .await
                .unwrap_or_else(|panic| {
                    Err(JobError::Retry(format!("the handler panicked: {panic}")))
                });
            let reason = result.as_ref().err().map(ToString::to_string);
            let ms = started.elapsed().as_millis();
            match self.jobs.finish(&job, result).await {
                Ok(JobOutcome::Done) => tracing::info!(ms, "job done"),
                Ok(JobOutcome::Retrying(delay)) => {
                    tracing::warn!(
                        ms,
                        reason,
                        retry_in_s = delay.as_secs(),
                        "job failed, will retry"
                    );
                }
                Ok(JobOutcome::Dead) => {
                    tracing::error!(ms, reason, "job failed for good, dead-lettered")
                }
                Ok(JobOutcome::Lost) => {
                    tracing::warn!(
                        ms,
                        "job outlived its lease, so this run's result was dropped"
                    );
                }
                Err(error) => {
                    tracing::error!(
                        ms,
                        ?error,
                        "cannot record the job's result; it runs again when its lease ends"
                    );
                }
            }
        }
        .instrument(span)
        .await;
    }

    async fn tidy_loop(self: Arc<Self>, mut stop: watch::Receiver<bool>) {
        loop {
            match self.jobs.tidy().await {
                Ok(tidied) if tidied != Tidied::default() => tracing::info!(
                    requeued = tidied.requeued,
                    dead = tidied.buried,
                    deleted = tidied.deleted,
                    "jobs tidied"
                ),
                Ok(_) => {}
                Err(error) => tracing::warn!(?error, "cannot tidy jobs"),
            }
            if !keep_going(&mut stop, TIDY_INTERVAL).await {
                break;
            }
        }
    }
}

/// Waits `interval`, cut short by a stop. `false` once stopping.
async fn keep_going(stop: &mut watch::Receiver<bool>, interval: Duration) -> bool {
    tokio::select! {
        () = tokio::time::sleep(interval) => !*stop.borrow(),
        changed = stop.changed() => changed.is_ok() && !*stop.borrow(),
    }
}
