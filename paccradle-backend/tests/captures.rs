//! Captures (M1-D5), end to end on a real database with photos in memory and the real vision
//! engine: an upload queues a vision run, the run saves the outline (or the reason it
//! couldn't), and corrections (a tap, an edited outline, a height) change it, with older runs
//! never overwriting newer ones. Needs OpenCV: run through `scripts/in-docker.sh`.
// The helpers below are test code too; clippy.toml only exempts `#[test]` functions.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use axum::http::{Method, StatusCode};
use common::photos::{DISC_CENTER_MM, DISC_DIAMETER_MM, disc_photo, photo_px, table_photo};
use common::{
    CAPTURES, Reply, TestApi, WEB_ORIGIN, auth_api, call, photo_part, settings_part, upload,
};
use paccradle_backend::jobs;
use paccradle_backend::models::capture::RunVision;
use paccradle_backend::models::job::{JobKind, JobOutcome};
use serde_json::{Value, json};
use sqlx::PgPool;

/// A new guest's access token.
async fn guest(api: &TestApi) -> String {
    let reply = api
        .send(call(Method::POST, "/api/client/v1/auth/guest").origin(WEB_ORIGIN))
        .await;
    reply.access_token()
}

/// Uploads `photo` in photo mode on A4 and returns the capture's id.
async fn capture(api: &TestApi, (token, photo): (&str, Vec<u8>)) -> String {
    let settings = json!({ "paper": "a4", "mode": "photo", "exif": { "focal_length_35mm": 26 } });
    let reply = upload(api, (token, &[settings_part(&settings), photo_part(photo)])).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{:?}", reply.body);
    reply.json()["capture_id"].as_str().unwrap().to_owned()
}

/// Runs every queued vision job, as the worker would, and returns how each ended.
async fn run_vision_jobs(api: &TestApi) -> Vec<JobOutcome> {
    let handlers = jobs::handlers(&api.state);
    let mut outcomes = Vec::new();
    while let Some(job) = api.state.jobs.claim(&[RunVision::NAME]).await.unwrap() {
        let result = handlers.run(&job).await;
        outcomes.push(api.state.jobs.finish(&job, result).await.unwrap());
    }
    outcomes
}

async fn get(api: &TestApi, (token, id): (&str, &str)) -> Reply {
    api.send(call(Method::GET, &format!("{CAPTURES}/{id}")).bearer(token))
        .await
}

async fn correct(api: &TestApi, (token, id, body): (&str, &str, Value)) -> Reply {
    api.send(
        call(Method::POST, &format!("{CAPTURES}/{id}/correct"))
            .bearer(token)
            .json(body),
    )
    .await
}

fn product(body: &Value) -> &Value {
    &body["result"]["products"][0]
}

fn assert_close(label: &str, (measured, expected, tolerance): (f64, f64, f64)) {
    assert!(
        (measured - expected).abs() <= tolerance,
        "{label}: {measured} vs {expected}"
    );
}

/// Where a tap on the disc's centre lands in the photo.
fn disc_tap() -> Value {
    let [point] = photo_px(&[DISC_CENTER_MM]).try_into().unwrap();
    json!(point)
}

#[sqlx::test]
async fn an_upload_is_measured_by_its_vision_run(pool: PgPool) {
    let api = auth_api(pool, &[]);
    let token = guest(&api).await;
    let id = capture(&api, (&token, disc_photo())).await;

    let pending = get(&api, (&token, &id)).await;
    assert_eq!(pending.status, StatusCode::OK);
    assert_eq!(pending.header("cache-control"), "no-store");
    assert_eq!(pending.header("api-version"), "1.0.0-beta.7");
    let body = pending.json();
    assert_eq!(body["status"], "pending");
    assert_eq!(body["result"], Value::Null);
    assert_eq!(body["error"], Value::Null);
    assert_eq!(body["settings"]["focal_length_35mm"], 26.0);
    let url = body["photo"]["url"].as_str().unwrap();
    assert!(
        url.starts_with(&format!("memory:///photos/{id}.jpg?")),
        "{url}"
    );

    assert_eq!(run_vision_jobs(&api).await, [JobOutcome::Done]);

    let body = get(&api, (&token, &id)).await.json();
    assert_eq!(body["status"], "done", "{body}");
    assert_eq!(body["error"], Value::Null);
    assert_eq!(body["result"]["level_used"], "L1");
    assert_eq!(body["result"]["paper"]["size"], "A4");
    assert_eq!(product(&body)["contour_source"], "detected");
    assert_close(
        "diameter",
        (
            product(&body)["diameter_mm"].as_f64().unwrap(),
            DISC_DIAMETER_MM,
            0.5,
        ),
    );
    // No height yet: the tall-product warning asks for one.
    assert_eq!(
        body["result"]["warnings"][0]["kind"],
        "tall_product_looks_bigger"
    );
    let row = sqlx::query!("SELECT run, confidence, level_used FROM captures")
        .fetch_one(&api.pool)
        .await
        .unwrap();
    assert_eq!((row.run, row.level_used.as_deref()), (1, Some("l1")));
    assert!(row.confidence.unwrap() > 0.5);
}

#[sqlx::test]
async fn a_photo_vision_cannot_read_fails_with_a_reason_the_editor_shows(pool: PgPool) {
    let api = auth_api(pool, &[]);
    let token = guest(&api).await;
    let id = capture(&api, (&token, table_photo())).await;

    assert_eq!(run_vision_jobs(&api).await, [JobOutcome::Done]);

    let body = get(&api, (&token, &id)).await.json();
    assert_eq!(body["status"], "failed");
    assert_eq!(body["result"], Value::Null);
    assert_eq!(
        body["error"],
        json!({
            "code": "CAPTURE_PAPER_NOT_FOUND",
            "message": "We couldn't find the paper. Put it on a darker surface and keep all 4 corners in the photo.",
            "meta": {}
        })
    );
    // An edit needs an outline.
    let reply = correct(
        &api,
        (
            &token,
            &id,
            json!({ "edited_contour_px": [[1, 1], [9, 1], [9, 9]] }),
        ),
    )
    .await;
    assert_eq!(
        (reply.status, reply.error_code().as_str()),
        (StatusCode::CONFLICT, "CAPTURE_NOT_READY")
    );
}

#[sqlx::test]
async fn a_tap_queues_a_new_run_and_the_last_outline_stays_until_it_lands(pool: PgPool) {
    let api = auth_api(pool, &[]);
    let token = guest(&api).await;
    let id = capture(&api, (&token, disc_photo())).await;
    run_vision_jobs(&api).await;
    let detected = get(&api, (&token, &id)).await.json()["result"].clone();

    let reply = correct(&api, (&token, &id, json!({ "tap_point_px": disc_tap() }))).await;

    assert_eq!(reply.status, StatusCode::ACCEPTED);
    let body = reply.json();
    assert_eq!(body["status"], "pending");
    assert_eq!(body["result"], detected, "kept while the run is pending");
    assert_eq!(body["settings"]["tap_point_px"], disc_tap());
    assert_eq!(run_vision_jobs(&api).await, [JobOutcome::Done]);
    let body = get(&api, (&token, &id)).await.json();
    assert_eq!(body["status"], "done");
    assert_close(
        "tapped diameter",
        (
            product(&body)["diameter_mm"].as_f64().unwrap(),
            DISC_DIAMETER_MM,
            0.5,
        ),
    );

    // A tap on bare paper finds nothing: the run fails, the last outline stays.
    let [bare] = photo_px(&[[30.0, 40.0]]).try_into().unwrap();
    let reply = correct(&api, (&token, &id, json!({ "tap_point_px": bare }))).await;
    assert_eq!(reply.status, StatusCode::ACCEPTED);
    run_vision_jobs(&api).await;
    let body = get(&api, (&token, &id)).await.json();
    assert_eq!(body["status"], "failed");
    assert_eq!(body["error"]["code"], "CAPTURE_NOTHING_AT_TAP");
    assert_eq!(product(&body)["contour_source"], "detected");
    let run = sqlx::query_scalar!("SELECT run FROM captures")
        .fetch_one(&api.pool)
        .await
        .unwrap();
    assert_eq!(run, 3);
}

#[sqlx::test]
async fn an_edit_is_measured_as_drawn_and_outdates_a_pending_run(pool: PgPool) {
    let api = auth_api(pool, &[]);
    let token = guest(&api).await;
    let id = capture(&api, (&token, disc_photo())).await;
    run_vision_jobs(&api).await;
    // A tap queues run 2…
    correct(&api, (&token, &id, json!({ "tap_point_px": disc_tap() }))).await;

    // …then the user draws a 60 × 40 mm box round the disc before it runs.
    let (x, y) = (DISC_CENTER_MM[0], DISC_CENTER_MM[1]);
    let outline = photo_px(&[
        [x - 30.0, y - 20.0],
        [x + 30.0, y - 20.0],
        [x + 30.0, y + 20.0],
        [x - 30.0, y + 20.0],
    ]);
    let reply = correct(&api, (&token, &id, json!({ "edited_contour_px": outline }))).await;

    assert_eq!(reply.status, StatusCode::OK, "{:?}", reply.body);
    let body = reply.json();
    assert_eq!(body["status"], "done");
    assert_eq!(product(&body)["contour_source"], "user");
    assert_close(
        "edited length",
        (product(&body)["bbox_mm"]["l"].as_f64().unwrap(), 60.0, 0.2),
    );
    // Run 2 is outdated: it leaves the user's outline alone.
    assert_eq!(run_vision_jobs(&api).await, [JobOutcome::Done]);
    let after = get(&api, (&token, &id)).await.json();
    assert_eq!(after["result"], body["result"]);

    // An outline that crosses itself is refused.
    let bow_tie = photo_px(&[[50.0, 50.0], [90.0, 90.0], [90.0, 50.0], [50.0, 90.0]]);
    let reply = correct(&api, (&token, &id, json!({ "edited_contour_px": bow_tie }))).await;
    assert_eq!(
        (reply.status, reply.error_code().as_str()),
        (StatusCode::BAD_REQUEST, "CAPTURE_OUTLINE_CROSSES_ITSELF")
    );
}

#[sqlx::test]
async fn a_height_applies_at_once_or_rides_a_new_run(pool: PgPool) {
    let api = auth_api(pool, &[]);
    let token = guest(&api).await;
    let id = capture(&api, (&token, disc_photo())).await;

    // Typed while the first run is pending: a new run takes it.
    let reply = correct(&api, (&token, &id, json!({ "height_mm": 95 }))).await;
    assert_eq!(reply.status, StatusCode::ACCEPTED);
    assert_eq!(reply.json()["settings"]["height_mm"], 95.0);
    assert_eq!(
        run_vision_jobs(&api).await,
        [JobOutcome::Done, JobOutcome::Done],
        "run 1 skipped, run 2 measured"
    );
    let body = get(&api, (&token, &id)).await.json();
    assert_eq!(body["status"], "done");
    assert_eq!(product(&body)["height_mm"], 95.0);
    assert_eq!(
        body["result"]["warnings"][0]["kind"],
        "outline_may_be_bigger"
    );

    // Typed once measured: applied at once, no new run.
    let reply = correct(&api, (&token, &id, json!({ "height_mm": 2 }))).await;
    assert_eq!(reply.status, StatusCode::OK);
    let body = reply.json();
    assert_eq!(product(&body)["height_mm"], 2.0);
    let warning = &body["result"]["warnings"][0];
    assert_eq!(warning["kind"], "outline_may_be_bigger");
    assert_eq!(warning["height_mm"], 2.0);
    assert!(warning["up_to_mm"].as_f64().unwrap() < 1.0, "{warning}");
    assert!(run_vision_jobs(&api).await.is_empty());

    let reply = correct(&api, (&token, &id, json!({ "height_mm": 0 }))).await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST);
    assert_eq!(reply.error_code(), "CAPTURE_INVALID_INPUT");
    assert_eq!(reply.json()["error"]["meta"]["field"], "height_mm");
    for body in [json!({}), json!({ "height_mm": 9, "tap_point_px": [1, 2] })] {
        let reply = correct(&api, (&token, &id, body)).await;
        assert_eq!(reply.error_code(), "INVALID_REQUEST");
    }
}

#[sqlx::test]
async fn captures_are_private(pool: PgPool) {
    let api = auth_api(pool, &[]);
    let owner = guest(&api).await;
    let other = guest(&api).await;
    let id = capture(&api, (&owner, disc_photo())).await;

    for token in [other.as_str()] {
        let reply = get(&api, (token, &id)).await;
        assert_eq!(
            (reply.status, reply.error_code().as_str()),
            (StatusCode::NOT_FOUND, "CAPTURE_NOT_FOUND")
        );
        let reply = correct(&api, (token, &id, json!({ "height_mm": 50 }))).await;
        assert_eq!(reply.error_code(), "CAPTURE_NOT_FOUND");
    }
    let unknown = "01a18a23-a088-72c1-984d-394c311f905b";
    assert_eq!(
        get(&api, (&owner, unknown)).await.error_code(),
        "CAPTURE_NOT_FOUND"
    );
    let reply = api
        .send(call(Method::GET, &format!("{CAPTURES}/{id}")))
        .await;
    assert_eq!(reply.error_code(), "AUTH_REQUIRED");
}

#[sqlx::test]
async fn a_deleted_photo_fails_its_run_and_refuses_taps(pool: PgPool) {
    let api = auth_api(pool, &[]);
    let token = guest(&api).await;
    let id = capture(&api, (&token, disc_photo())).await;
    // The purge got to it first.
    sqlx::query!("UPDATE captures SET storage_key = NULL")
        .execute(&api.pool)
        .await
        .unwrap();

    run_vision_jobs(&api).await;

    let body = get(&api, (&token, &id)).await.json();
    assert_eq!(body["status"], "failed");
    assert_eq!(body["error"]["code"], "CAPTURE_PHOTO_DELETED");
    assert_eq!(body["photo"], Value::Null);
    let reply = correct(&api, (&token, &id, json!({ "tap_point_px": disc_tap() }))).await;
    assert_eq!(
        (reply.status, reply.error_code().as_str()),
        (StatusCode::GONE, "CAPTURE_PHOTO_DELETED")
    );
}

#[sqlx::test]
async fn a_run_whose_job_gives_up_is_marked_failed(pool: PgPool) {
    let api = auth_api(pool, &[]);
    let token = guest(&api).await;
    let id = capture(&api, (&token, disc_photo())).await;
    api.storage.set_available(false);

    // Retried while it has attempts left: still pending.
    assert!(matches!(
        run_vision_jobs(&api).await[..],
        [JobOutcome::Retrying(_)]
    ));
    assert_eq!(get(&api, (&token, &id)).await.json()["status"], "pending");

    // The last attempt fails too.
    sqlx::query!("UPDATE jobs SET run_at = now(), max_attempts = attempts + 1")
        .execute(&api.pool)
        .await
        .unwrap();
    assert_eq!(run_vision_jobs(&api).await, [JobOutcome::Dead]);
    let body = get(&api, (&token, &id)).await.json();
    assert_eq!(body["status"], "failed");
    assert_eq!(body["error"]["code"], "CAPTURE_PROCESSING_FAILED");
}

#[sqlx::test]
async fn taps_count_against_the_hourly_vision_runs(pool: PgPool) {
    let api = auth_api(pool, &[("RATE_LIMIT_UPLOADS_PER_HOUR", "2")]);
    let token = guest(&api).await;
    let id = capture(&api, (&token, disc_photo())).await;

    let reply = correct(&api, (&token, &id, json!({ "tap_point_px": disc_tap() }))).await;
    assert_eq!(reply.status, StatusCode::ACCEPTED);
    let reply = correct(&api, (&token, &id, json!({ "tap_point_px": disc_tap() }))).await;
    assert_eq!(reply.error_code(), "RATE_LIMITED");
    // Heights and edits cost no vision run.
    let reply = correct(&api, (&token, &id, json!({ "height_mm": 30 }))).await;
    assert_eq!(reply.status, StatusCode::ACCEPTED, "a run was pending");
}
