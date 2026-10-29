//! `/readyz` (M1-D2): ready only while the database answers; the deploy waits on it.
// The helpers below are test code too; clippy.toml only exempts `#[test]` functions.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use axum::http::StatusCode;
use common::{app, app_on, get, send};
use sqlx::PgPool;

#[sqlx::test]
async fn readyz_is_ok_while_the_database_answers(pool: PgPool) {
    let reply = send(&app_on(pool, &[]), get("/readyz")).await;

    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.json()["status"], "ok");
    assert_eq!(reply.json()["version"], env!("CARGO_PKG_VERSION"));
}

#[tokio::test]
async fn readyz_names_the_database_when_it_is_down() {
    let reply = send(&app(&[]), get("/readyz")).await;

    assert_eq!(reply.status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(reply.error_code(), "SERVICE_UNAVAILABLE");
    assert_eq!(reply.json()["error"]["meta"]["dependency"], "database");
    // The driver's text (addresses, users) stays in the logs.
    let body = String::from_utf8_lossy(&reply.body).into_owned();
    assert!(
        !body.contains("127.0.0.1") && !body.contains("nobody"),
        "{body}"
    );
}
