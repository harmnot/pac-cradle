//! The shared middleware (M1-D1) on test routes: timeouts, panics, body limits, rejected
//! bodies, compression and the per-client rate limit.
// The helpers below are test code too; clippy.toml only exempts `#[test]` functions.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::time::Duration;

use axum::Router;
use axum::body::{Body, Bytes};
use axum::http::Request;
use axum::http::StatusCode;
use axum::http::header::SET_COOKIE;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, http};
use common::{config, send};
use paccradle_backend::config::AppConfig;
use paccradle_backend::middleware;
use serde_json::Value;

fn big_json() -> Json<Value> {
    Json(serde_json::json!({ "contour_mm": vec![[12.34, 56.78]; 400] }))
}

fn test_app(config: &AppConfig) -> Router {
    let routes = Router::new()
        .route(
            "/slow",
            get(|| async {
                tokio::time::sleep(Duration::from_secs(5)).await;
                "late"
            }),
        )
        .route(
            "/panic",
            get(|| async {
                let cells: Vec<u32> = Vec::new();
                cells[3].to_string()
            }),
        )
        .route(
            "/bytes",
            post(|body: Bytes| async move { body.len().to_string() }),
        )
        .route(
            "/json",
            post(|Json(body): Json<Value>| async move { Json(body) }),
        )
        .route("/big", get(|| async { big_json() }))
        .route(
            "/cookie",
            get(|| async {
                ([(SET_COOKIE, "refresh=secret; HttpOnly")], big_json()).into_response()
            }),
        );
    middleware::stack(routes, config)
}

fn post_body(path: &str, content_type: Option<&str>, body: impl Into<Body>) -> Request<Body> {
    let mut request = Request::post(path);
    if let Some(content_type) = content_type {
        request = request.header("content-type", content_type);
    }
    request.body(body.into()).unwrap()
}

#[tokio::test]
async fn a_request_running_past_the_limit_gets_request_timeout() {
    let mut config = config(&[]);
    config.request_timeout = Duration::from_millis(50);
    let reply = send(&test_app(&config), common::get("/slow")).await;

    assert_eq!(reply.status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(reply.error_code(), "REQUEST_TIMEOUT");
    assert!(!reply.header("x-request-id").is_empty());
}

#[tokio::test]
async fn a_panic_becomes_internal_error_without_its_text() {
    let reply = send(&test_app(&config(&[])), common::get("/panic")).await;

    assert_eq!(reply.status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(reply.error_code(), "INTERNAL_ERROR");
    let body = String::from_utf8_lossy(&reply.body).into_owned();
    assert!(!body.contains("index out of bounds"), "{body}");
}

#[tokio::test]
async fn a_body_over_the_limit_gets_payload_too_large() {
    let app = test_app(&config(&[("HTTP_BODY_LIMIT_BYTES", "1024")]));

    let fits = send(&app, post_body("/bytes", None, vec![b'x'; 1024])).await;
    assert_eq!(fits.status, StatusCode::OK);

    let too_big = send(&app, post_body("/bytes", None, vec![b'x'; 1025])).await;
    assert_eq!(too_big.status, StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(too_big.error_code(), "PAYLOAD_TOO_LARGE");
}

#[tokio::test]
async fn rejected_bodies_get_our_codes_not_framework_text() {
    let app = test_app(&config(&[]));

    let not_json = send(&app, post_body("/json", Some("application/json"), "{oops")).await;
    assert_eq!(not_json.status, StatusCode::BAD_REQUEST);
    assert_eq!(not_json.error_code(), "INVALID_REQUEST");
    assert_eq!(not_json.header("content-type"), "application/json");

    let no_type = send(&app, post_body("/json", None, "{}")).await;
    assert_eq!(no_type.status, StatusCode::UNSUPPORTED_MEDIA_TYPE);
    assert_eq!(no_type.error_code(), "UNSUPPORTED_MEDIA_TYPE");
}

fn accepting_gzip(path: &str) -> Request<Body> {
    Request::get(path)
        .header("accept-encoding", "gzip")
        .body(Body::empty())
        .unwrap()
}

#[tokio::test]
async fn responses_are_compressed_unless_they_set_a_cookie() {
    let app = test_app(&config(&[]));

    let plain = send(&app, accepting_gzip("/big")).await;
    assert_eq!(plain.header("content-encoding"), "gzip");

    let with_cookie = send(&app, accepting_gzip("/cookie")).await;
    assert_eq!(with_cookie.header("content-encoding"), "");
    assert!(!with_cookie.header("set-cookie").is_empty());
}

fn from_client(ip: &str) -> Request<Body> {
    Request::get("/big")
        .header("x-real-ip", ip)
        .body(Body::empty())
        .unwrap()
}

#[tokio::test]
async fn each_client_address_gets_its_own_rate_limit() {
    let app = test_app(&config(&[
        ("RATE_LIMIT_PER_MINUTE", "2"),
        ("RATE_LIMIT_CLIENT_IP_HEADER", "x-real-ip"),
    ]));

    for _ in 0..2 {
        assert_eq!(
            send(&app, from_client("203.0.113.7")).await.status,
            StatusCode::OK
        );
    }
    let limited = send(&app, from_client("203.0.113.7")).await;
    assert_eq!(limited.status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(limited.error_code(), "RATE_LIMITED");
    // 2 a minute refill one every 30 s.
    assert_eq!(limited.header("retry-after"), "30");
    assert_eq!(limited.json()["error"]["meta"]["retry_after_seconds"], 30);
    assert_eq!(
        limited.json()["error"]["message"],
        "Too many requests. Please try again in 30 s."
    );

    assert_eq!(
        send(&app, from_client("198.51.100.4")).await.status,
        StatusCode::OK
    );
}

#[tokio::test]
async fn the_client_address_header_counts_only_when_configured() {
    // Without RATE_LIMIT_CLIENT_IP_HEADER a caller can't dodge the limit by inventing addresses.
    let app = test_app(&config(&[("RATE_LIMIT_PER_MINUTE", "1")]));

    assert_eq!(
        send(&app, from_client("203.0.113.7")).await.status,
        StatusCode::OK
    );
    let reply = send(&app, from_client("198.51.100.4")).await;
    assert_eq!(reply.status, StatusCode::TOO_MANY_REQUESTS);
}

#[test]
fn compression_is_skipped_for_cookie_responses_only() {
    use tower_http::compression::predicate::Predicate;

    let cookie = http::Response::builder()
        .header(SET_COOKIE, "a=b")
        .body(Body::empty())
        .unwrap();
    let plain = http::Response::builder().body(Body::empty()).unwrap();

    assert!(!middleware::compression::NoCookie.should_compress(&cookie));
    assert!(middleware::compression::NoCookie.should_compress(&plain));
}
