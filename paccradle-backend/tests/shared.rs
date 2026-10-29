//! Shared helpers (M1-D1): the rate limiter's buckets, message placeholders, error codes.
// The helpers below are test code too; clippy.toml only exempts `#[test]` functions.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::num::NonZeroU32;
use std::time::{Duration, Instant};

use axum::http::StatusCode;
use paccradle_backend::error::ErrorCode;
use paccradle_backend::locales::{Locale, fill};
use paccradle_backend::shared::rate_limiter::RateLimiter;
use serde_json::{Map, Value, json};

fn per_minute(n: u32) -> NonZeroU32 {
    NonZeroU32::new(n).unwrap()
}

#[test]
fn a_bucket_spends_its_minute_then_refills_evenly() {
    let limiter = RateLimiter::new(per_minute(60));
    let start = Instant::now();

    for _ in 0..60 {
        assert_eq!(limiter.check("a", start), Ok(()));
    }
    let wait = limiter.check("a", start).unwrap_err();
    assert!((wait.as_secs_f64() - 1.0).abs() < 1e-9, "{wait:?}");

    // One a second comes back.
    assert_eq!(limiter.check("a", start + Duration::from_secs(1)), Ok(()));
    assert!(limiter.check("a", start + Duration::from_secs(1)).is_err());
}

#[test]
fn keys_have_separate_buckets() {
    let limiter = RateLimiter::new(per_minute(1));
    let now = Instant::now();

    assert_eq!(limiter.check(1, now), Ok(()));
    assert!(limiter.check(1, now).is_err());
    assert_eq!(limiter.check(2, now), Ok(()));
}

#[test]
fn idle_keys_are_swept_once_there_are_many() {
    let limiter = RateLimiter::new(per_minute(10));
    let start = Instant::now();
    for key in 0..10_000 {
        limiter.check(key, start).unwrap();
    }
    assert_eq!(limiter.tracked_keys(), 10_000);

    // A minute later every bucket is full again, so the next check sweeps them all out.
    limiter.check(-1, start + Duration::from_secs(60)).unwrap();
    assert_eq!(limiter.tracked_keys(), 1);
}

#[test]
fn placeholders_take_meta_values() {
    let meta: Map<String, Value> = json!({ "retry_after_seconds": 12, "field": "height_mm" })
        .as_object()
        .unwrap()
        .clone();

    assert_eq!(fill("Wait {retry_after_seconds} s.", &meta), "Wait 12 s.");
    assert_eq!(fill("{field} is wrong", &meta), "height_mm is wrong");
    assert_eq!(fill("No {value} here", &meta), "No {value} here");
    assert_eq!(fill("Open { brace", &meta), "Open { brace");
    assert_eq!(fill("", &meta), "");
}

#[test]
fn every_error_code_has_a_message_and_a_matching_fallback() {
    let codes = ErrorCode::ALL;
    let mut names: Vec<&str> = codes.iter().map(|code| code.as_str()).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), codes.len(), "codes are unique");
    for code in codes {
        for locale in Locale::ALL {
            assert!(!locale.error_template(code).is_empty(), "{code}");
        }
        let text = code.as_str();
        assert!(
            text.chars().all(|c| c.is_ascii_uppercase() || c == '_'),
            "{text}"
        );
    }
    // Framework errors map onto the same codes.
    assert_eq!(
        ErrorCode::from_status(StatusCode::UNPROCESSABLE_ENTITY),
        ErrorCode::InvalidRequest
    );
    assert_eq!(
        ErrorCode::from_status(StatusCode::NOT_FOUND),
        ErrorCode::RouteNotFound
    );
    assert_eq!(
        ErrorCode::from_status(StatusCode::BAD_GATEWAY),
        ErrorCode::Internal
    );
}
