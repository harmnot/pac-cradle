//! The endpoint sweep (M1-D9): one table of every route the API serves and who may call it,
//! checked three ways. Against the route files: no route is missing from the table. Against
//! the router: each one answers, lets in only its callers and carries its surface's
//! `api-version`. Against the dated Postman contracts in `postman/`: each one has a contract,
//! and every request a contract documents is a route. A new route fails here until it is in
//! the table and in a contract; what it does is tested in its domain's own test file.
// The helpers below are test code too; clippy.toml only exempts `#[test]` functions.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use Access::{Admin, Public, Session};
use axum::http::{Method, StatusCode};
use common::{Reply, TestApi, auth_api, call};
use paccradle_backend::shared::api_surface::ApiSurface;
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

/// Who may call an endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Access {
    /// Anyone: the probes, sign-in and refresh, the client's version.
    Public,
    /// A client session, a guest's too.
    Session,
    /// A signed-in admin, on the back office.
    Admin,
}

/// An endpoint: its method, its path as the router spells it (`{name}` for a parameter) and
/// who may call it.
type Endpoint = (&'static str, &'static str, Access);

/// Every endpoint the API serves (CLAUDE.md §8.3).
const ENDPOINTS: &[Endpoint] = &[
    // Probes (M1-D1, D2).
    ("GET", "/healthz", Public),
    ("GET", "/readyz", Public),
    // Client: version (D1) and sign-in (D3).
    ("GET", "/api/client/v1/version", Public),
    ("POST", "/api/client/v1/auth/guest", Public),
    ("POST", "/api/client/v1/auth/email-link", Public),
    ("POST", "/api/client/v1/auth/email-link/sign-in", Public),
    ("POST", "/api/client/v1/auth/password/sign-in", Public),
    (
        "POST",
        "/api/client/v1/auth/passkey/sign-in/options",
        Public,
    ),
    ("POST", "/api/client/v1/auth/passkey/sign-in", Public),
    ("POST", "/api/client/v1/auth/refresh", Public),
    ("POST", "/api/client/v1/auth/sign-out", Public),
    // Client: the account (D3, D8).
    ("GET", "/api/client/v1/me", Session),
    ("PUT", "/api/client/v1/me/password", Session),
    ("POST", "/api/client/v1/me/passkeys/options", Session),
    ("POST", "/api/client/v1/me/passkeys", Session),
    // Client: captures (D4, D5).
    ("POST", "/api/client/v1/captures", Session),
    ("GET", "/api/client/v1/captures/{capture_id}", Session),
    (
        "POST",
        "/api/client/v1/captures/{capture_id}/correct",
        Session,
    ),
    // Client: designs (D6).
    ("GET", "/api/client/v1/designs", Session),
    ("POST", "/api/client/v1/designs", Session),
    ("GET", "/api/client/v1/designs/{design_id}", Session),
    ("PATCH", "/api/client/v1/designs/{design_id}", Session),
    // Client: exports (D7).
    ("GET", "/api/client/v1/designs/{design_id}/exports", Session),
    (
        "POST",
        "/api/client/v1/designs/{design_id}/exports",
        Session,
    ),
    ("GET", "/api/client/v1/exports/{export_id}", Session),
    // Back office: sign-in (D3), then everything behind the admin guard.
    ("POST", "/api/admin/v1/auth/passkey/sign-in/options", Public),
    ("POST", "/api/admin/v1/auth/passkey/sign-in", Public),
    ("POST", "/api/admin/v1/auth/refresh", Public),
    ("POST", "/api/admin/v1/auth/sign-out", Public),
    ("GET", "/api/admin/v1/version", Admin),
    ("GET", "/api/admin/v1/me", Admin),
];

/// Contract requests that document a path with no route on purpose (the `ROUTE_NOT_FOUND`
/// answer).
const CONTRACTED_MISSES: &[&str] = &["GET /api/client/v1/nowhere"];

/// Answers that mean the router has no such endpoint.
const NO_ENDPOINT: &[&str] = &["ROUTE_NOT_FOUND", "METHOD_NOT_ALLOWED"];

const ADMIN_ORIGIN: &str = "http://localhost:3001";

fn backend_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

/// `METHOD /path`, as both sides of each comparison spell an endpoint.
fn key((method, path): (&str, &str)) -> String {
    format!("{method} {path}")
}

fn table() -> BTreeSet<String> {
    ENDPOINTS
        .iter()
        .map(|(method, path, _)| key((method, path)))
        .collect()
}

/// The error code of an answer, if it is an error body.
fn error_code(reply: &Reply) -> Option<String> {
    let body: Value = serde_json::from_slice(&reply.body).ok()?;
    body["error"]["code"].as_str().map(str::to_owned)
}

/// The `*_route.rs` files under `dir`, at any depth.
fn route_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files.extend(route_files(&path));
        } else if path.to_string_lossy().ends_with("_route.rs") {
            files.push(path);
        }
    }
    files
}

/// The text from `text`'s start up to the `)` closing a call whose `(` came just before it,
/// skipping string literals.
fn call_arguments(text: &str) -> &str {
    let mut depth = 1;
    let mut in_string = false;
    let mut escaped = false;
    for (at, c) in text.char_indices() {
        match (in_string, escaped, c) {
            (true, true, _) => escaped = false,
            (true, false, '\\') => escaped = true,
            (true, false, '"') => in_string = false,
            (true, false, _) => {}
            (false, _, '"') => in_string = true,
            (false, _, '(') => depth += 1,
            (false, _, ')') if depth == 1 => return &text[..at],
            (false, _, ')') => depth -= 1,
            (false, _, _) => {}
        }
    }
    panic!("a .route( call never closes");
}

/// The path and methods of every `.route(path, get(…).post(…))` call in a route file.
fn route_calls(text: &str) -> Vec<(String, Vec<&'static str>)> {
    text.match_indices(".route(")
        .map(|(at, found)| {
            let arguments = call_arguments(&text[at + found.len()..]);
            let path = arguments
                .split('"')
                .nth(1)
                .expect("a route's first argument is its path, spelled out");
            let methods = ["get", "post", "put", "patch", "delete"]
                .into_iter()
                .filter(|method| {
                    arguments
                        .match_indices(&format!("{method}("))
                        .any(|(at, _)| {
                            let before = arguments[..at].chars().next_back();
                            !before.is_some_and(|c| c.is_alphanumeric() || c == '_')
                        })
                })
                .map(|method| match method {
                    "get" => "GET",
                    "post" => "POST",
                    "put" => "PUT",
                    "patch" => "PATCH",
                    _ => "DELETE",
                })
                .collect();
            (path.to_owned(), methods)
        })
        .collect()
}

/// Every endpoint the route files define, under the prefix its folder is mounted on:
/// `ops/` on none, `client/` and `admin/` on their surface's, `auth/` and `meta/` on both.
fn routes_in_source() -> BTreeSet<String> {
    let handlers = backend_dir().join("src/handlers");
    let mut routes = BTreeSet::new();
    for file in route_files(&handlers) {
        let relative = file.strip_prefix(&handlers).unwrap();
        let folder = relative.components().next().unwrap().as_os_str();
        let prefixes = match folder.to_str().unwrap() {
            "ops" => vec![String::new()],
            "client" => vec![ApiSurface::Client.prefix()],
            "admin" => vec![ApiSurface::Admin.prefix()],
            "auth" | "meta" => ApiSurface::ALL.map(ApiSurface::prefix).to_vec(),
            other => panic!(
                "{}: routes in an unknown folder, {other}",
                relative.display()
            ),
        };
        for (path, methods) in route_calls(&fs::read_to_string(&file).unwrap()) {
            assert!(
                !methods.is_empty(),
                "{}: {path} has no method",
                relative.display()
            );
            for prefix in &prefixes {
                for method in &methods {
                    routes.insert(key((method, &format!("{prefix}{path}"))));
                }
            }
        }
    }
    routes
}

/// A contract request's URL as a router path: no `{{base_url}}`, no query, and `{{name}}`
/// variables as `{name}` parameters.
fn contract_path(url: &Value) -> String {
    let raw = url
        .get("raw")
        .and_then(Value::as_str)
        .or_else(|| url.as_str())
        .expect("a request has a URL");
    let path = raw.trim_start_matches("{{base_url}}");
    let path = path.split('?').next().unwrap_or_default();
    path.replace("{{", "{").replace("}}", "}")
}

/// `METHOD /path` of every request in a Postman collection's items, at any depth.
fn contract_requests(items: &[Value], into: &mut Vec<String>) {
    for item in items {
        if let Some(children) = item["item"].as_array() {
            contract_requests(children, into);
            continue;
        }
        let request = &item["request"];
        let method = request["method"].as_str().expect("a request has a method");
        into.push(key((method, &contract_path(&request["url"]))));
    }
}

/// Every request documented by `postman/*.postman_collection.json`, checking each file is
/// named `<topic>-YYYY-MM-DD.postman_collection.json` (`code-style.md`).
fn contracts() -> BTreeSet<String> {
    let mut requests = Vec::new();
    for entry in fs::read_dir(backend_dir().join("postman")).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_str().unwrap().to_owned();
        let Some(stem) = name.strip_suffix(".postman_collection.json") else {
            continue;
        };
        let date = stem.rsplitn(4, '-').take(3).collect::<Vec<_>>();
        let dated = date.len() == 3
            && date
                .iter()
                .all(|part| part.chars().all(|c| c.is_ascii_digit()))
            && date.iter().map(|part| part.len()).eq([2, 2, 4]);
        assert!(dated, "{name}: contract files end with their date");
        let collection: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        contract_requests(collection["item"].as_array().unwrap(), &mut requests);
    }
    requests.into_iter().collect()
}

#[test]
fn the_table_lists_every_route_the_route_files_define() {
    let in_source = routes_in_source();
    let table = table();
    let missing: Vec<_> = in_source.difference(&table).collect();
    let stale: Vec<_> = table.difference(&in_source).collect();
    assert!(
        missing.is_empty(),
        "routes missing from ENDPOINTS: {missing:#?}"
    );
    assert!(
        stale.is_empty(),
        "ENDPOINTS no route file defines: {stale:#?}"
    );
}

#[test]
fn every_endpoint_has_a_dated_contract_and_every_contract_request_is_an_endpoint() {
    let contracts = contracts();
    let table = table();
    let uncontracted: Vec<_> = table.difference(&contracts).collect();
    assert!(
        uncontracted.is_empty(),
        "endpoints without a contract: {uncontracted:#?}"
    );
    let unknown: Vec<_> = contracts
        .iter()
        .filter(|request| {
            !table.contains(*request) && !CONTRACTED_MISSES.contains(&request.as_str())
        })
        .collect();
    assert!(
        unknown.is_empty(),
        "contract requests with no route: {unknown:#?}"
    );
}

/// The path of `endpoint` with a new id for each parameter.
fn concrete(path: &str) -> String {
    path.split('/')
        .map(|segment| match segment.starts_with('{') {
            true => Uuid::now_v7().to_string(),
            false => segment.to_owned(),
        })
        .collect::<Vec<_>>()
        .join("/")
}

/// The `api-version` an answer from `path` carries: its surface's, none for the probes.
fn expected_version(path: &str) -> &'static str {
    ApiSurface::ALL
        .into_iter()
        .find(|surface| path.starts_with(&surface.prefix()))
        .map_or("", ApiSurface::version)
}

async fn send(api: &TestApi, (method, path, token): (&str, &str, Option<&str>)) -> Reply {
    let request = call(Method::from_bytes(method.as_bytes()).unwrap(), path);
    match token {
        Some(token) => api.send(request.bearer(token)).await,
        None => api.send(request).await,
    }
}

#[sqlx::test]
async fn every_endpoint_answers_and_lets_in_only_its_callers(pool: PgPool) {
    let api = auth_api(pool, &[("ADMIN_WEB_URL", ADMIN_ORIGIN)]);
    let guest = api
        .send(call(Method::POST, "/api/client/v1/auth/guest"))
        .await;
    assert_eq!(guest.status, StatusCode::CREATED);
    let app_token = guest.access_token();

    for &(method, template, access) in ENDPOINTS {
        let path = concrete(template);
        let endpoint = key((method, template));

        let anonymous = send(&api, (method, &path, None)).await;
        assert_eq!(
            anonymous.header("api-version"),
            expected_version(template),
            "{endpoint}"
        );
        let code = error_code(&anonymous);
        match access {
            Public => assert!(
                !code
                    .as_deref()
                    .is_some_and(|code| { NO_ENDPOINT.contains(&code) || code == "AUTH_REQUIRED" }),
                "{endpoint}: {} {code:?}",
                anonymous.status
            ),
            Session | Admin => {
                assert_eq!(anonymous.status, StatusCode::UNAUTHORIZED, "{endpoint}");
                assert_eq!(code.as_deref(), Some("AUTH_REQUIRED"), "{endpoint}");
            }
        }

        // The web app's token is enough for client endpoints and opens nothing on the back
        // office.
        let with_app_token = send(&api, (method, &path, Some(&app_token))).await;
        let code = error_code(&with_app_token);
        match access {
            Public => {}
            Session => assert!(
                !code
                    .as_deref()
                    .is_some_and(|code| { NO_ENDPOINT.contains(&code) || code == "AUTH_REQUIRED" }),
                "{endpoint}: {} {code:?}",
                with_app_token.status
            ),
            Admin => assert_eq!(code.as_deref(), Some("AUTH_REQUIRED"), "{endpoint}"),
        }
    }

    for miss in CONTRACTED_MISSES {
        let (method, path) = miss.split_once(' ').unwrap();
        let reply = send(&api, (method, path, None)).await;
        assert_eq!(reply.error_code(), "ROUTE_NOT_FOUND", "{miss}");
    }
}
