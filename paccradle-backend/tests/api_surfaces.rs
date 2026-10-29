//! The URL scheme (M1-D1): unversioned probes, then the client and admin surfaces, each with
//! its own prefix, semver header, CORS origin and (for admin) guard.
// The helpers below are test code too; clippy.toml only exempts `#[test]` functions.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use common::{WEB_ORIGIN, app, get, send};
use paccradle_backend::shared::api_surface::ApiSurface;
use uuid::Uuid;

#[tokio::test]
async fn healthz_answers_ok_with_the_release_version_even_without_a_database() {
    let reply = send(&app(&[]), get("/healthz")).await;

    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.json()["status"], "ok");
    assert_eq!(reply.json()["version"], env!("CARGO_PKG_VERSION"));
    // Probes sit outside both surfaces, so they carry no contract version.
    assert_eq!(reply.header("api-version"), "");
}

#[tokio::test]
async fn every_response_gets_the_apis_own_uuid_v7_request_id() {
    let app = app(&[]);
    let sent_by_caller = "not-ours";
    let request = Request::get("/healthz")
        .header("x-request-id", sent_by_caller)
        .body(Body::empty())
        .unwrap();
    let first = send(&app, request).await;
    let second = send(&app, get("/nowhere")).await;

    let first_id = Uuid::parse_str(first.header("x-request-id")).expect("a UUID");
    let second_id = Uuid::parse_str(second.header("x-request-id")).expect("a UUID, errors too");
    assert_eq!(first_id.get_version_num(), 7);
    assert_ne!(first_id, second_id);
}

#[test]
fn each_surface_has_a_semver_version_and_its_major_in_the_prefix() {
    for surface in ApiSurface::ALL {
        let version = surface.version();
        let (core, pre_release) = version.split_once('-').unwrap_or((version, ""));
        let numbers: Vec<u64> = core.split('.').map(|n| n.parse().unwrap()).collect();
        assert_eq!(numbers.len(), 3, "{version} is major.minor.patch");
        assert!(
            pre_release.split('.').all(|part| !part.is_empty()),
            "{version}"
        );
        assert_eq!(surface.major(), numbers[0].to_string());
        assert_eq!(
            surface.prefix(),
            format!("/api/{}/v{}", surface.name(), numbers[0])
        );
    }
    assert_eq!(ApiSurface::Client.prefix(), "/api/client/v1");
    assert_eq!(ApiSurface::Admin.prefix(), "/api/admin/v1");
}

#[tokio::test]
async fn client_version_names_the_surface_and_its_contract_version() {
    let reply = send(&app(&[]), get("/api/client/v1/version")).await;

    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.json()["surface"], "client");
    assert_eq!(reply.json()["version"], ApiSurface::Client.version());
    assert_eq!(reply.header("api-version"), ApiSurface::Client.version());
}

#[tokio::test]
async fn the_admin_surface_refuses_everyone_but_signed_in_admins() {
    let app = app(&[]);
    // A real admin route and an unknown one answer alike, so the routes stay hidden. Admins
    // sign in with a passkey (tests/auth.rs).
    for path in ["/api/admin/v1/version", "/api/admin/v1/users"] {
        let reply = send(&app, get(path)).await;
        assert_eq!(reply.status, StatusCode::UNAUTHORIZED, "{path}");
        assert_eq!(reply.error_code(), "AUTH_REQUIRED");
        assert_eq!(
            reply.json()["error"]["message"],
            "Please sign in to continue."
        );
        assert_eq!(reply.header("api-version"), ApiSurface::Admin.version());
    }
}

#[tokio::test]
async fn unknown_paths_get_route_not_found() {
    let app = app(&[]);
    for path in [
        "/nowhere",
        "/api/v1/health",
        "/api/client/v2/version",
        "/api/client/v1/nowhere",
    ] {
        let reply = send(&app, get(path)).await;
        assert_eq!(reply.status, StatusCode::NOT_FOUND, "{path}");
        assert_eq!(reply.error_code(), "ROUTE_NOT_FOUND", "{path}");
        assert_eq!(reply.header("content-type"), "application/json");
    }
    // Inside a surface, even a 404 says which contract answered.
    let reply = send(&app, get("/api/client/v1/nowhere")).await;
    assert_eq!(reply.header("api-version"), ApiSurface::Client.version());
}

#[tokio::test]
async fn a_wrong_method_gets_method_not_allowed_as_json() {
    let request = Request::post("/healthz").body(Body::empty()).unwrap();
    let reply = send(&app(&[]), request).await;

    assert_eq!(reply.status, StatusCode::METHOD_NOT_ALLOWED);
    assert_eq!(reply.error_code(), "METHOD_NOT_ALLOWED");
    assert!(
        reply.header("allow").contains("GET"),
        "the allow header is kept"
    );
}

#[tokio::test]
async fn errors_answer_in_english_when_the_callers_language_is_not_offered_yet() {
    let request = Request::get("/nowhere")
        .header("accept-language", "id-ID,id;q=0.9")
        .body(Body::empty())
        .unwrap();
    let reply = send(&app(&[]), request).await;

    assert_eq!(
        reply.json()["error"]["message"],
        "There is nothing at this address."
    );
    assert_eq!(reply.json()["error"]["meta"], serde_json::json!({}));
}

fn preflight(path: &str, origin: &str) -> Request<Body> {
    Request::builder()
        .method(Method::OPTIONS)
        .uri(path)
        .header("origin", origin)
        .header("access-control-request-method", "POST")
        .header(
            "access-control-request-headers",
            "content-type,authorization",
        )
        .body(Body::empty())
        .unwrap()
}

#[tokio::test]
async fn the_client_surface_answers_cors_only_for_the_web_app() {
    let app = app(&[]);

    let ours = send(&app, preflight("/api/client/v1/version", WEB_ORIGIN)).await;
    assert_eq!(ours.status, StatusCode::OK);
    assert_eq!(ours.header("access-control-allow-origin"), WEB_ORIGIN);
    assert_eq!(ours.header("access-control-allow-credentials"), "true");

    let other = send(
        &app,
        preflight("/api/client/v1/version", "https://evil.example"),
    )
    .await;
    assert_eq!(other.header("access-control-allow-origin"), "");

    let simple = Request::get("/api/client/v1/version")
        .header("origin", WEB_ORIGIN)
        .body(Body::empty())
        .unwrap();
    let simple = send(&app, simple).await;
    assert_eq!(simple.header("access-control-allow-origin"), WEB_ORIGIN);
    let exposed = simple.header("access-control-expose-headers");
    assert!(exposed.contains("x-request-id") && exposed.contains("api-version"));
}

#[tokio::test]
async fn the_admin_surface_answers_cors_only_for_the_back_office() {
    // On localhost like the web app, so one passkey domain (WEBAUTHN_RP_ID) covers both.
    let back_office = "http://localhost:3001";

    // Unset ADMIN_WEB_URL: no browser origin may call the admin API, not even the web app.
    let closed = app(&[]);
    let reply = send(&closed, preflight("/api/admin/v1/version", WEB_ORIGIN)).await;
    assert_eq!(reply.header("access-control-allow-origin"), "");

    // Set: its preflight passes before the guard, which then refuses the real request.
    let open = app(&[("ADMIN_WEB_URL", back_office)]);
    let reply = send(&open, preflight("/api/admin/v1/version", back_office)).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.header("access-control-allow-origin"), back_office);
    let reply = send(&open, preflight("/api/admin/v1/version", WEB_ORIGIN)).await;
    assert_eq!(reply.header("access-control-allow-origin"), "");
}
