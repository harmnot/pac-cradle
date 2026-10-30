//! The 30-day photo purge (M1-D4) on a real database: due photos leave storage and their
//! captures forget them, the daily `purge_photos` job is scheduled once a day and retried
//! through a storage outage, and a long purge hands over to another run.
// The helpers below are test code too; clippy.toml only exempts `#[test]` functions.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::sync::Arc;
use std::time::Duration;

use axum::http::Method;
use common::{TestApi, WEB_ORIGIN, auth_api, call};
use paccradle_backend::jobs::{self, JobHandlers};
use paccradle_backend::models::capture::PurgePhotos;
use paccradle_backend::models::job::{JobKind, JobOutcome};
use paccradle_backend::repository::capture::PgCaptureRepository;
use paccradle_backend::repository::database::PgDatabase;
use paccradle_backend::service::privacy::{
    DefaultPrivacyService, PURGE_BATCH, PrivacyProviders, PrivacyService, PurgeReport,
};
use paccradle_backend::third_party::storage::{NewObject, ObjectStorage};
use sqlx::PgPool;
use uuid::Uuid;

/// A guest's id.
async fn guest(api: &TestApi) -> Uuid {
    let reply = api
        .send(call(Method::POST, "/api/client/v1/auth/guest").origin(WEB_ORIGIN))
        .await;
    reply.json()["account"]["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap()
}

/// Adds `count` captures for `user` with stored photos, uploaded `days_ago` (so due once
/// that is over 30).
async fn captures(api: &TestApi, (user, count, days_ago): (Uuid, i64, f64)) -> Vec<String> {
    let keys: Vec<String> = sqlx::query_scalar!(
        r#"
        INSERT INTO captures (id, user_id, mode, paper, input, storage_key, created_at, purge_after)
        SELECT gen_random_uuid(), $1, 'photo', 'a4', '{"paper": "a4", "mode": "photo"}',
               'photos/' || gen_random_uuid() || '.jpg',
               now() - make_interval(secs => $3::float8 * 86400),
               now() - make_interval(secs => ($3::float8 - 30) * 86400)
        FROM generate_series(1, $2::int8)
        RETURNING storage_key AS "storage_key!"
        "#,
        user,
        count,
        days_ago,
    )
    .fetch_all(&api.pool)
    .await
    .unwrap();
    for key in &keys {
        api.storage
            .put(NewObject {
                key: key.clone(),
                bytes: b"photo".to_vec(),
                content_type: "image/jpeg",
            })
            .await
            .unwrap();
    }
    keys
}

async fn stored_keys(pool: &PgPool) -> i64 {
    sqlx::query_scalar!(
        r#"SELECT count(*) AS "count!" FROM captures WHERE storage_key IS NOT NULL"#
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn captures_left(pool: &PgPool) -> i64 {
    sqlx::query_scalar!(r#"SELECT count(*) AS "count!" FROM captures"#)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test]
async fn photos_past_their_30_days_are_deleted_and_forgotten(pool: PgPool) {
    let api = auth_api(pool, &[]);
    let user = guest(&api).await;
    let due = captures(&api, (user, 2, 31.0)).await;
    let kept = captures(&api, (user, 1, 29.0)).await;

    let report = api.state.privacy.purge_due_photos().await.unwrap();

    assert_eq!(
        report,
        PurgeReport {
            deleted: 2,
            continued: false
        }
    );
    assert_eq!(api.storage.keys(), kept);
    for key in &due {
        assert!(api.storage.object(key).is_none());
    }
    // The captures stay (outlines and sizes are kept); only the photo is gone.
    assert_eq!(captures_left(&api.pool).await, 3);
    assert_eq!(stored_keys(&api.pool).await, 1);
    // Nothing more to do.
    assert_eq!(
        api.state.privacy.purge_due_photos().await.unwrap(),
        PurgeReport::default()
    );
}

/// Claims the next `purge_photos` job and runs it as the worker would.
async fn run_purge_job(api: &TestApi, handlers: &JobHandlers) -> Option<JobOutcome> {
    let job = api.state.jobs.claim(&[PurgePhotos::NAME]).await.unwrap()?;
    let result = handlers.run(&job).await;
    Some(api.state.jobs.finish(&job, result).await.unwrap())
}

#[sqlx::test]
async fn the_daily_job_is_scheduled_once_a_day_and_retried_through_an_outage(pool: PgPool) {
    let api = auth_api(pool, &[]);
    let user = guest(&api).await;
    captures(&api, (user, 3, 40.0)).await;
    let schedules = jobs::schedules().unwrap();
    let purge = schedules
        .iter()
        .find(|schedule| schedule.kind() == PurgePhotos::NAME)
        .unwrap();
    assert_eq!(purge.every, Duration::from_secs(24 * 60 * 60));
    let handlers = jobs::handlers(&api.state);
    assert!(handlers.kinds().contains(&PurgePhotos::NAME));

    // The first check queues a run; while it is queued, no second one.
    assert!(api.state.jobs.schedule(purge).await.unwrap());
    assert!(!api.state.jobs.schedule(purge).await.unwrap());

    // Storage is down: the run fails and waits to be retried.
    api.storage.set_available(false);
    assert!(matches!(
        run_purge_job(&api, &handlers).await,
        Some(JobOutcome::Retrying(_))
    ));
    assert!(
        !api.state.jobs.schedule(purge).await.unwrap(),
        "still queued"
    );
    assert_eq!(stored_keys(&api.pool).await, 3);

    // Back up: the retry deletes them.
    api.storage.set_available(true);
    sqlx::query!("UPDATE jobs SET run_at = now()")
        .execute(&api.pool)
        .await
        .unwrap();
    assert_eq!(run_purge_job(&api, &handlers).await, Some(JobOutcome::Done));
    assert_eq!(stored_keys(&api.pool).await, 0);
    assert!(api.storage.keys().is_empty());

    // Done today: the next run is due a day after this one finished.
    assert!(!api.state.jobs.schedule(purge).await.unwrap());
    sqlx::query!("UPDATE jobs SET finished_at = now() - interval '25 hours'")
        .execute(&api.pool)
        .await
        .unwrap();
    assert!(api.state.jobs.schedule(purge).await.unwrap());
}

#[sqlx::test]
async fn a_long_purge_hands_over_to_another_run(pool: PgPool) {
    let api = auth_api(pool.clone(), &[]);
    let user = guest(&api).await;
    let total = PURGE_BATCH + 20;
    captures(&api, (user, total, 45.0)).await;
    // A run that is out of time after its first batch.
    let privacy = DefaultPrivacyService::new(PrivacyProviders {
        db: Arc::new(PgDatabase::new(pool.clone())),
        captures: Arc::new(PgCaptureRepository::new()),
        storage: api.storage.clone(),
        jobs: api.state.jobs.clone(),
    })
    .with_time_budget(Duration::ZERO);

    let first = privacy.purge_due_photos().await.unwrap();

    assert_eq!(
        first,
        PurgeReport {
            deleted: u64::try_from(PURGE_BATCH).unwrap(),
            continued: true
        }
    );
    let queued = sqlx::query_scalar!(
        r#"SELECT count(*) AS "count!" FROM jobs WHERE kind = 'purge_photos' AND status = 'queued'"#
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(queued, 1, "the next run carries on");

    let second = privacy.purge_due_photos().await.unwrap();
    assert_eq!(
        second,
        PurgeReport {
            deleted: 20,
            continued: false
        }
    );
    assert_eq!(stored_keys(&pool).await, 0);
    assert!(api.storage.keys().is_empty());
}

#[sqlx::test]
async fn a_photo_already_gone_from_storage_is_still_forgotten(pool: PgPool) {
    let api = auth_api(pool, &[]);
    let user = guest(&api).await;
    let keys = captures(&api, (user, 1, 31.0)).await;
    api.storage.delete(&keys[0]).await.unwrap();

    let report = api.state.privacy.purge_due_photos().await.unwrap();

    assert_eq!(report.deleted, 1);
    assert_eq!(stored_keys(&api.pool).await, 0);
}
