//! The job queue and worker (M1-D2): outbox inserts, SKIP LOCKED claims, backoff, dead
//! letters, leases, housekeeping, and the worker running real handlers.
// The helpers below are test code too; clippy.toml only exempts `#[test]` functions.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use paccradle_backend::jobs::{JobHandler, JobHandlers, Worker};
use paccradle_backend::models::job::{JobError, JobKind, JobOutcome, NewJob, Schedule, Tidied};
use paccradle_backend::repository::job::PgJobRepository;
use paccradle_backend::service::job::{DefaultJobService, JobService, RetryPolicy};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tokio::sync::watch;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Greeting {
    name: String,
}

/// A test kind with the default 5 attempts.
struct Greet;

impl JobKind for Greet {
    const NAME: &'static str = "greet";
    type Payload = Greeting;
}

/// A test kind with 2 attempts.
struct Fragile;

impl JobKind for Fragile {
    const NAME: &'static str = "fragile";
    const MAX_ATTEMPTS: i32 = 2;
    type Payload = Greeting;
}

fn queue(pool: &PgPool) -> DefaultJobService {
    DefaultJobService::new(Arc::new(PgJobRepository::new(pool.clone())))
}

fn greeting(name: &str) -> Greeting {
    Greeting {
        name: name.to_owned(),
    }
}

async fn add(queue: &dyn JobService, pool: &PgPool, job: NewJob) -> Uuid {
    let mut conn = pool.acquire().await.unwrap();
    queue.enqueue(&mut conn, &job).await.unwrap();
    job.id
}

async fn status(pool: &PgPool, id: Uuid) -> (String, i32, Option<String>) {
    sqlx::query_as("SELECT status, attempts, last_error FROM jobs WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn make_due(pool: &PgPool, id: Uuid) {
    sqlx::query("UPDATE jobs SET run_at = now() WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
        .unwrap();
}

async fn expire_lease(pool: &PgPool, id: Uuid) {
    sqlx::query("UPDATE jobs SET locked_until = now() - interval '1 second' WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
        .unwrap();
}

const GREET: &[&str] = &[Greet::NAME];
const BOTH: &[&str] = &[Greet::NAME, Fragile::NAME];

#[sqlx::test]
async fn a_job_exists_only_if_the_callers_transaction_commits(pool: PgPool) {
    let queue = queue(&pool);

    let mut rolled_back = pool.begin().await.unwrap();
    let job = NewJob::of::<Greet>(&greeting("never")).unwrap();
    queue.enqueue(&mut rolled_back, &job).await.unwrap();
    rolled_back.rollback().await.unwrap();
    assert!(queue.claim(GREET).await.unwrap().is_none());

    let mut committed = pool.begin().await.unwrap();
    let job = NewJob::of::<Greet>(&greeting("Ana")).unwrap();
    queue.enqueue(&mut committed, &job).await.unwrap();
    committed.commit().await.unwrap();

    let claimed = queue
        .claim(GREET)
        .await
        .unwrap()
        .expect("the committed job");
    assert_eq!(claimed.id, job.id);
    assert_eq!(claimed.kind, "greet");
    assert_eq!(claimed.payload["name"], "Ana");
    assert_eq!((claimed.attempts, claimed.max_attempts), (1, 5));
    assert_eq!(status(&pool, job.id).await.0, "running");
}

#[sqlx::test]
async fn a_claim_takes_the_oldest_due_job_of_a_kind_it_knows(pool: PgPool) {
    let queue = queue(&pool);
    let first = add(
        &queue,
        &pool,
        NewJob::of::<Greet>(&greeting("first")).unwrap(),
    )
    .await;
    let other_kind = add(
        &queue,
        &pool,
        NewJob::of::<Fragile>(&greeting("other")).unwrap(),
    )
    .await;
    let later = NewJob::of::<Greet>(&greeting("later"))
        .unwrap()
        .after(Duration::from_secs(3600));
    add(&queue, &pool, later).await;
    let second = add(
        &queue,
        &pool,
        NewJob::of::<Greet>(&greeting("second")).unwrap(),
    )
    .await;

    assert_eq!(queue.claim(GREET).await.unwrap().unwrap().id, first);
    assert_eq!(queue.claim(GREET).await.unwrap().unwrap().id, second);
    // The delayed one isn't due, and the other kind isn't asked for (a newer release's kind
    // waits instead of failing).
    assert!(queue.claim(GREET).await.unwrap().is_none());
    assert!(queue.claim(&[]).await.unwrap().is_none());
    assert_eq!(status(&pool, other_kind).await.0, "queued");
}

#[sqlx::test]
async fn workers_claiming_at_once_never_get_the_same_job(pool: PgPool) {
    let queue: Arc<dyn JobService> = Arc::new(queue(&pool));
    let mut added = HashSet::new();
    for n in 0..30 {
        added.insert(
            add(
                queue.as_ref(),
                &pool,
                NewJob::of::<Greet>(&greeting(&n.to_string())).unwrap(),
            )
            .await,
        );
    }

    let claimers: Vec<_> = (0..5)
        .map(|_| {
            let queue = Arc::clone(&queue);
            tokio::spawn(async move {
                let mut got = Vec::new();
                while let Some(job) = queue.claim(GREET).await.unwrap() {
                    got.push(job.id);
                }
                got
            })
        })
        .collect();
    let mut claimed = Vec::new();
    for claimer in claimers {
        claimed.extend(claimer.await.unwrap());
    }

    assert_eq!(claimed.len(), 30, "every job claimed exactly once");
    assert_eq!(claimed.into_iter().collect::<HashSet<_>>(), added);
}

#[sqlx::test]
async fn a_failing_job_backs_off_then_dies_on_its_last_attempt(pool: PgPool) {
    let queue = queue(&pool);
    let id = add(
        &queue,
        &pool,
        NewJob::of::<Fragile>(&greeting("Ana")).unwrap(),
    )
    .await;

    let job = queue.claim(BOTH).await.unwrap().unwrap();
    let outcome = queue
        .finish(&job, Err(JobError::Retry("the provider is down".into())))
        .await;
    assert_eq!(
        outcome.unwrap(),
        JobOutcome::Retrying(Duration::from_secs(10))
    );
    assert_eq!(
        status(&pool, id).await,
        ("queued".into(), 1, Some("the provider is down".into()))
    );
    // Not due for 10 s.
    assert!(queue.claim(BOTH).await.unwrap().is_none());

    make_due(&pool, id).await;
    let job = queue.claim(BOTH).await.unwrap().unwrap();
    assert_eq!(job.attempts, 2);
    let outcome = queue
        .finish(&job, Err(JobError::Retry("still down".into())))
        .await;
    assert_eq!(outcome.unwrap(), JobOutcome::Dead);
    assert_eq!(
        status(&pool, id).await,
        ("dead".into(), 2, Some("still down".into()))
    );
}

#[sqlx::test]
async fn giving_up_dead_letters_at_once_and_success_clears_the_error(pool: PgPool) {
    let queue = queue(&pool);
    let doomed = add(
        &queue,
        &pool,
        NewJob::of::<Greet>(&greeting("doomed")).unwrap(),
    )
    .await;
    let fine = add(
        &queue,
        &pool,
        NewJob::of::<Greet>(&greeting("fine")).unwrap(),
    )
    .await;

    let job = queue.claim(GREET).await.unwrap().unwrap();
    let outcome = queue
        .finish(&job, Err(JobError::GiveUp("the design is gone".into())))
        .await;
    assert_eq!(outcome.unwrap(), JobOutcome::Dead);
    assert_eq!(
        status(&pool, doomed).await,
        ("dead".into(), 1, Some("the design is gone".into()))
    );

    let job = queue.claim(GREET).await.unwrap().unwrap();
    queue
        .finish(&job, Err(JobError::Retry("blip".into())))
        .await
        .unwrap();
    make_due(&pool, fine).await;
    let job = queue.claim(GREET).await.unwrap().unwrap();
    assert_eq!(queue.finish(&job, Ok(())).await.unwrap(), JobOutcome::Done);
    assert_eq!(status(&pool, fine).await, ("done".into(), 2, None));
}

#[sqlx::test]
async fn a_run_that_outlives_its_lease_loses_its_result(pool: PgPool) {
    let queue = queue(&pool);
    let id = add(
        &queue,
        &pool,
        NewJob::of::<Greet>(&greeting("slow")).unwrap(),
    )
    .await;

    let slow_run = queue.claim(GREET).await.unwrap().unwrap();
    expire_lease(&pool, id).await;
    let tidied = queue.tidy().await.unwrap();
    assert_eq!(
        tidied,
        Tidied {
            requeued: 1,
            buried: 0,
            deleted: 0
        }
    );
    let (state, _, error) = status(&pool, id).await;
    assert_eq!(state, "queued");
    assert!(error.unwrap().contains("lease ran out"));

    let next_run = queue.claim(GREET).await.unwrap().unwrap();
    assert_eq!(next_run.attempts, 2);
    // The first run ends late: its result no longer applies.
    assert_eq!(
        queue.finish(&slow_run, Ok(())).await.unwrap(),
        JobOutcome::Lost
    );
    assert_eq!(status(&pool, id).await.0, "running");
    assert_eq!(
        queue.finish(&next_run, Ok(())).await.unwrap(),
        JobOutcome::Done
    );
}

#[sqlx::test]
async fn housekeeping_buries_expired_last_attempts_and_deletes_old_done_jobs(pool: PgPool) {
    let queue = queue(&pool);
    let last_try = add(
        &queue,
        &pool,
        NewJob::of::<Fragile>(&greeting("x")).unwrap(),
    )
    .await;
    sqlx::query("UPDATE jobs SET attempts = 1 WHERE id = $1")
        .bind(last_try)
        .execute(&pool)
        .await
        .unwrap();
    let job = queue.claim(BOTH).await.unwrap().unwrap();
    assert_eq!(job.attempts, 2);
    expire_lease(&pool, last_try).await;

    let old = add(
        &queue,
        &pool,
        NewJob::of::<Greet>(&greeting("old")).unwrap(),
    )
    .await;
    let recent = add(
        &queue,
        &pool,
        NewJob::of::<Greet>(&greeting("recent")).unwrap(),
    )
    .await;
    for id in [old, recent] {
        sqlx::query("UPDATE jobs SET status = 'done', finished_at = now() WHERE id = $1")
            .bind(id)
            .execute(&pool)
            .await
            .unwrap();
    }
    sqlx::query("UPDATE jobs SET finished_at = now() - interval '8 days' WHERE id = $1")
        .bind(old)
        .execute(&pool)
        .await
        .unwrap();

    let tidied = queue.tidy().await.unwrap();
    assert_eq!(
        tidied,
        Tidied {
            requeued: 0,
            buried: 1,
            deleted: 1
        }
    );
    assert_eq!(status(&pool, last_try).await.0, "dead");
    assert_eq!(status(&pool, recent).await.0, "done");
    let left: i64 = sqlx::query_scalar("SELECT count(*) FROM jobs WHERE id = $1")
        .bind(old)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(left, 0);
}

#[test]
fn retries_wait_longer_each_time_up_to_the_cap() {
    let policy = RetryPolicy::default();
    let waits: Vec<u64> = (1..=6)
        .map(|attempt| policy.delay(attempt).as_secs())
        .collect();
    assert_eq!(waits, [10, 20, 40, 80, 160, 320]);
    assert_eq!(policy.delay(40), Duration::from_secs(3600));
    assert_eq!(policy.delay(0), Duration::from_secs(10));
}

/// Fails each name's first run, then succeeds; panics on "boom".
struct Flaky {
    seen: Arc<Mutex<Vec<String>>>,
}

impl JobHandler for Flaky {
    type Kind = Greet;

    async fn run(&self, payload: Greeting) -> Result<(), JobError> {
        assert_ne!(payload.name, "boom", "the handler crashed");
        let mut seen = self.seen.lock().unwrap();
        let first_time = !seen.contains(&payload.name);
        seen.push(payload.name);
        if first_time {
            return Err(JobError::Retry("first runs fail".into()));
        }
        Ok(())
    }
}

async fn wait_until_finished(pool: &PgPool, ids: &[Uuid]) {
    for _ in 0..500 {
        let open: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM jobs WHERE id = ANY($1) AND status NOT IN ('done', 'dead')",
        )
        .bind(ids)
        .fetch_one(pool)
        .await
        .unwrap();
        if open == 0 {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("the worker didn't finish the jobs in 5 s");
}

#[sqlx::test]
async fn the_worker_runs_retries_and_survives_a_panicking_handler(pool: PgPool) {
    let no_wait = RetryPolicy {
        first: Duration::ZERO,
        max: Duration::ZERO,
    };
    let queue: Arc<dyn JobService> = Arc::new(queue(&pool).with_retry(no_wait));
    let mut ids = Vec::new();
    for name in ["Ana", "Budi", "Chen", "boom"] {
        ids.push(
            add(
                queue.as_ref(),
                &pool,
                NewJob::of::<Greet>(&greeting(name)).unwrap(),
            )
            .await,
        );
    }

    let seen = Arc::new(Mutex::new(Vec::new()));
    let handlers = JobHandlers::new().with(Flaky {
        seen: Arc::clone(&seen),
    });
    let (stop_sender, stop) = watch::channel(false);
    let loops = Worker::new(Arc::clone(&queue), handlers)
        .poll_every(Duration::from_millis(10))
        .start(2, stop);
    assert_eq!(loops.len(), 3, "2 claim loops and housekeeping");

    wait_until_finished(&pool, &ids).await;
    stop_sender.send(true).unwrap();
    for worker_loop in loops {
        tokio::time::timeout(Duration::from_secs(2), worker_loop)
            .await
            .expect("the loops stop promptly")
            .unwrap();
    }

    for id in &ids[..3] {
        assert_eq!(status(&pool, *id).await, ("done".into(), 2, None));
    }
    let (state, attempts, error) = status(&pool, ids[3]).await;
    assert_eq!((state.as_str(), attempts), ("dead", 5));
    assert!(error.unwrap().contains("panicked"));
    let mut runs = seen.lock().unwrap().clone();
    runs.sort();
    assert_eq!(runs, ["Ana", "Ana", "Budi", "Budi", "Chen", "Chen"]);
}

#[sqlx::test]
async fn no_workers_or_no_handlers_claim_nothing(pool: PgPool) {
    let queue: Arc<dyn JobService> = Arc::new(queue(&pool));
    let id = add(
        queue.as_ref(),
        &pool,
        NewJob::of::<Greet>(&greeting("Ana")).unwrap(),
    )
    .await;
    let (stop_sender, stop) = watch::channel(false);

    assert!(
        Worker::new(Arc::clone(&queue), JobHandlers::new())
            .start(0, stop.clone())
            .is_empty()
    );
    let loops = Worker::new(Arc::clone(&queue), JobHandlers::new())
        .poll_every(Duration::from_millis(10))
        .start(2, stop);
    assert_eq!(loops.len(), 1, "housekeeping only");
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(status(&pool, id).await.0, "queued");

    stop_sender.send(true).unwrap();
    for worker_loop in loops {
        worker_loop.await.unwrap();
    }
}

/// M1-D4: a scheduled kind is queued by housekeeping as soon as the worker starts, then run;
/// a kind the worker has no handler for is never scheduled.
#[sqlx::test]
async fn the_worker_queues_scheduled_jobs_it_can_run(pool: PgPool) {
    let queue: Arc<dyn JobService> = Arc::new(queue(&pool));
    let day = Duration::from_secs(24 * 60 * 60);
    let schedules = vec![
        Schedule::of::<Greet>((&greeting("daily"), day)).unwrap(),
        Schedule::of::<Fragile>((&greeting("never"), day)).unwrap(),
    ];
    let seen = Arc::new(Mutex::new(vec!["daily".to_owned()]));
    let handlers = JobHandlers::new().with(Flaky {
        seen: Arc::clone(&seen),
    });
    let (stop_sender, stop) = watch::channel(false);
    let loops = Worker::new(Arc::clone(&queue), handlers)
        .with_schedules(schedules)
        .poll_every(Duration::from_millis(10))
        .start(1, stop);

    let mut kinds = Vec::new();
    for _ in 0..500 {
        kinds = sqlx::query_scalar!("SELECT kind FROM jobs WHERE status = 'done'")
            .fetch_all(&pool)
            .await
            .unwrap();
        if !kinds.is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    stop_sender.send(true).unwrap();
    for worker_loop in loops {
        worker_loop.await.unwrap();
    }
    assert_eq!(kinds, ["greet"]);
    let all = sqlx::query_scalar!("SELECT kind FROM jobs")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(all, ["greet"], "no handler, no schedule");
}

/// Records the payloads it was told failed for good (M1-D5).
struct Doomed {
    failed: Arc<Mutex<Vec<String>>>,
}

impl JobHandler for Doomed {
    type Kind = Fragile;

    async fn run(&self, payload: Greeting) -> Result<(), JobError> {
        if payload.name == "hopeless" {
            return Err(JobError::GiveUp("never".into()));
        }
        Err(JobError::Retry("not yet".into()))
    }

    async fn failed_for_good(&self, payload: Greeting) {
        self.failed.lock().unwrap().push(payload.name);
    }
}

/// M1-D5: a handler hears when its job fails for good (it gave up, or its last attempt
/// failed), and not before.
#[sqlx::test]
async fn a_handler_hears_when_its_job_fails_for_good(pool: PgPool) {
    let queue = queue(&pool);
    let failed = Arc::new(Mutex::new(Vec::new()));
    let handlers = JobHandlers::new().with(Doomed {
        failed: Arc::clone(&failed),
    });
    for name in ["hopeless", "patient"] {
        add(
            &queue,
            &pool,
            NewJob::of::<Fragile>(&greeting(name)).unwrap(),
        )
        .await;
    }
    let mut outcomes = Vec::new();
    for _ in 0..4 {
        sqlx::query("UPDATE jobs SET run_at = now()")
            .execute(&pool)
            .await
            .unwrap();
        while let Some(job) = queue.claim(&[Fragile::NAME]).await.unwrap() {
            let result = handlers.run(&job).await;
            outcomes.push(queue.finish(&job, result).await.unwrap());
        }
    }
    // "patient" fails its first attempt (retried), then its second and last.
    assert_eq!(outcomes.len(), 3);
    assert_eq!(*failed.lock().unwrap(), ["hopeless", "patient"]);
}
