//! Helpers shared by the API tests: a config from a list of variables, the app with or without
//! a database, and one-shot requests through the router without opening a port.

// Each test file uses only some helpers, and helpers are test code too (clippy.toml only
// exempts `#[test]` functions).
#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]

pub mod authenticator;
pub mod designs;
pub mod photos;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::{Body, Bytes};
use axum::http::header::{AUTHORIZATION, CONTENT_TYPE, COOKIE, ORIGIN, SET_COOKIE};
use axum::http::{HeaderMap, Method, Request, StatusCode};
use paccradle_backend::config::AppConfig;
use paccradle_backend::error::AppError;
use paccradle_backend::jobs;
use paccradle_backend::models::email_link::SendEmailLink;
use paccradle_backend::models::job::JobKind;
use paccradle_backend::repository::database;
use paccradle_backend::shared::future::BoxFuture;
use paccradle_backend::third_party::email::{EmailMessage, EmailSender};
use paccradle_backend::third_party::storage::MemoryStorage;
use paccradle_backend::{AppState, Providers};
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

/// The web app's origin in tests.
pub const WEB_ORIGIN: &str = "http://localhost:3000";

/// A config from `vars`; anything not listed takes its default.
pub fn config(vars: &[(&str, &str)]) -> AppConfig {
    let vars: HashMap<String, String> = vars
        .iter()
        .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
        .collect();
    AppConfig::from_lookup(|name| vars.get(name).cloned()).expect("test config is valid")
}

/// A database address nothing listens on (port 9, discard), for tests that never reach the
/// database or check what happens when it's down.
pub const NO_DATABASE: &str = "postgres://nobody:nothing@127.0.0.1:9/none";

/// The whole API with `vars`, and a database that is never there.
pub fn app(vars: &[(&str, &str)]) -> Router {
    let config = config(&[&[("DATABASE_URL", NO_DATABASE)], vars].concat());
    let pool = database::connect_lazy(&config.database).expect("the URL parses");
    paccradle_backend::app(AppState::new(config, pool).expect("the providers start"))
}

/// The whole API with `vars` on `pool` (a `#[sqlx::test]` database).
pub fn app_on(pool: PgPool, vars: &[(&str, &str)]) -> Router {
    paccradle_backend::app(AppState::new(config(vars), pool).expect("the providers start"))
}

/// A response, read to the end.
pub struct Reply {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Bytes,
}

impl Reply {
    /// The body as JSON.
    pub fn json(&self) -> Value {
        serde_json::from_slice(&self.body).expect("body is JSON")
    }

    /// A header as text, or "" when missing.
    pub fn header(&self, name: &str) -> &str {
        self.headers
            .get(name)
            .map(|value| value.to_str().expect("header is text"))
            .unwrap_or_default()
    }

    /// The error code of an error body.
    pub fn error_code(&self) -> String {
        self.json()["error"]["code"]
            .as_str()
            .expect("error body has a code")
            .to_owned()
    }

    /// The `Set-Cookie` header for cookie `name`, whole, or "" when not set.
    pub fn set_cookie(&self, name: &str) -> String {
        self.headers
            .get_all(SET_COOKIE)
            .iter()
            .map(|value| value.to_str().unwrap().to_owned())
            .find(|value| value.starts_with(&format!("{name}=")))
            .unwrap_or_default()
    }

    /// The value cookie `name` is set to, if it is set to one.
    pub fn cookie(&self, name: &str) -> Option<String> {
        let header = self.set_cookie(name);
        let value = header.split(';').next()?.split_once('=')?.1;
        (!value.is_empty()).then(|| value.to_owned())
    }

    /// The `access_token` of a session body.
    pub fn access_token(&self) -> String {
        self.json()["access_token"]
            .as_str()
            .expect("a session body")
            .to_owned()
    }
}

/// Sends `request` through `app` (a clone, so state such as rate limits carries over).
pub async fn send(app: &Router, request: Request<Body>) -> Reply {
    let response = app
        .clone()
        .oneshot(request)
        .await
        .expect("router never fails");
    let (parts, body) = response.into_parts();
    let body = axum::body::to_bytes(body, usize::MAX)
        .await
        .expect("body can be read");
    Reply {
        status: parts.status,
        headers: parts.headers,
        body,
    }
}

/// A request being built: `call(Method::POST, path).json(…).bearer(…).send_to(&api)`.
pub struct Call {
    method: Method,
    path: String,
    headers: Vec<(axum::http::HeaderName, String)>,
    body: Option<Value>,
}

/// Starts a request.
pub fn call(method: Method, path: &str) -> Call {
    Call {
        method,
        path: path.to_owned(),
        headers: Vec::new(),
        body: None,
    }
}

impl Call {
    /// A JSON body.
    pub fn json(mut self, body: Value) -> Self {
        self.body = Some(body);
        self
    }

    /// `Authorization: Bearer <token>`.
    pub fn bearer(self, token: &str) -> Self {
        self.header(AUTHORIZATION, &format!("Bearer {token}"))
    }

    /// `Cookie: <name>=<value>`.
    pub fn cookie(self, name: &str, value: &str) -> Self {
        self.header(COOKIE, &format!("{name}={value}"))
    }

    /// `Origin: <origin>`.
    pub fn origin(self, origin: &str) -> Self {
        self.header(ORIGIN, origin)
    }

    /// Any header.
    pub fn header(mut self, name: axum::http::HeaderName, value: &str) -> Self {
        self.headers.push((name, value.to_owned()));
        self
    }

    /// The request.
    pub fn build(self) -> Request<Body> {
        let mut builder = Request::builder().method(self.method).uri(self.path);
        for (name, value) in self.headers {
            builder = builder.header(name, value);
        }
        match self.body {
            Some(body) => builder
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
            None => builder.body(Body::empty()).unwrap(),
        }
    }
}

/// An email the API sent.
pub type EmailSent = EmailMessage;

/// Emails the API sent, kept instead of sent.
#[derive(Default)]
pub struct Outbox {
    sent: Mutex<Vec<EmailMessage>>,
}

impl Outbox {
    /// Everything sent so far, oldest first.
    pub fn sent(&self) -> Vec<EmailMessage> {
        self.sent.lock().unwrap().clone()
    }
}

impl EmailSender for Outbox {
    fn send<'a>(&'a self, message: &'a EmailMessage) -> BoxFuture<'a, Result<(), AppError>> {
        Box::pin(async move {
            self.sent.lock().unwrap().push(message.clone());
            Ok(())
        })
    }
}

/// The API on a test database, with its state, its outbox and its photo store.
pub struct TestApi {
    pub app: Router,
    pub state: AppState,
    pub outbox: Arc<Outbox>,
    pub storage: Arc<MemoryStorage>,
    pub pool: PgPool,
}

/// The API on `pool` with `vars`. Sign-in routes get a high per-address limit unless `vars`
/// set one (every test request comes from the same unknown address).
pub fn auth_api(pool: PgPool, vars: &[(&str, &str)]) -> TestApi {
    let config = config(&[&[("RATE_LIMIT_AUTH_PER_MINUTE", "10000")], vars].concat());
    let outbox = Arc::new(Outbox::default());
    let storage = Arc::new(MemoryStorage::new());
    let providers = Providers {
        email: outbox.clone(),
        storage: storage.clone(),
    };
    let state = AppState::with_providers(config, pool.clone(), providers);
    TestApi {
        app: paccradle_backend::app(state.clone()),
        state,
        outbox,
        storage,
        pool,
    }
}

impl TestApi {
    /// Sends `call`.
    pub async fn send(&self, call: Call) -> Reply {
        send(&self.app, call.build()).await
    }

    /// Runs the queued email jobs, as the worker would, and returns what they sent.
    pub async fn deliver_emails(&self) -> Vec<EmailMessage> {
        let before = self.outbox.sent().len();
        let handlers = jobs::handlers(&self.state);
        while let Some(job) = self.state.jobs.claim(&[SendEmailLink::NAME]).await.unwrap() {
            let result = handlers.run(&job).await;
            self.state.jobs.finish(&job, result).await.unwrap();
        }
        self.outbox.sent()[before..].to_vec()
    }
}

/// The token in a sign-in email's link.
pub fn link_token(email: &EmailMessage) -> String {
    let (_, after) = email
        .text
        .split_once("#token=")
        .expect("the email has a link");
    after.split_whitespace().next().unwrap().to_owned()
}

/// A `GET` request with no headers.
pub fn get(path: &str) -> Request<Body> {
    Request::get(path)
        .body(Body::empty())
        .expect("valid request")
}

/// The boundary of the multipart bodies tests send.
pub const BOUNDARY: &str = "paccradle-test-boundary";

/// `POST /api/client/v1/captures` (M1-D4).
pub const CAPTURES: &str = "/api/client/v1/captures";

/// One part of a multipart body: its name and bytes, and a file name for a file.
pub struct Part<'a> {
    pub name: &'a str,
    pub file_name: Option<&'a str>,
    pub bytes: Vec<u8>,
}

pub fn settings_part(settings: &Value) -> Part<'static> {
    Part {
        name: "capture",
        file_name: None,
        bytes: serde_json::to_vec(settings).unwrap(),
    }
}

pub fn photo_part(bytes: Vec<u8>) -> Part<'static> {
    Part {
        name: "photo",
        file_name: Some("photo.jpg"),
        bytes,
    }
}

pub fn multipart(parts: &[Part<'_>]) -> Vec<u8> {
    let mut body = Vec::new();
    for part in parts {
        body.extend(format!("--{BOUNDARY}\r\n").as_bytes());
        let disposition = match part.file_name {
            Some(file) => format!(
                "Content-Disposition: form-data; name=\"{}\"; filename=\"{file}\"\r\n\
                 Content-Type: image/jpeg\r\n",
                part.name
            ),
            None => format!(
                "Content-Disposition: form-data; name=\"{}\"\r\n\
                 Content-Type: application/json\r\n",
                part.name
            ),
        };
        body.extend(disposition.as_bytes());
        body.extend(b"\r\n");
        body.extend(&part.bytes);
        body.extend(b"\r\n");
    }
    body.extend(format!("--{BOUNDARY}--\r\n").as_bytes());
    body
}

/// `POST /captures` with `parts`, as `token`.
pub async fn upload(api: &TestApi, (token, parts): (&str, &[Part<'_>])) -> Reply {
    let request = Request::builder()
        .method(Method::POST)
        .uri(CAPTURES)
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .header(
            CONTENT_TYPE,
            format!("multipart/form-data; boundary={BOUNDARY}"),
        )
        .body(Body::from(multipart(parts)))
        .unwrap();
    send(&api.app, request).await
}

/// A signed-in account.
pub struct Account {
    pub token: String,
    pub id: Uuid,
}

/// A new account for `email`, signed up through its confirmation link.
pub async fn account(api: &TestApi, email: &str) -> Account {
    let reply = api
        .send(call(Method::POST, "/api/client/v1/auth/email-link").json(json!({ "email": email })))
        .await;
    assert_eq!(reply.status, StatusCode::ACCEPTED);
    let sent = api.deliver_emails().await;
    let reply = api
        .send(
            call(Method::POST, "/api/client/v1/auth/email-link/sign-in")
                .json(json!({ "token": link_token(&sent[0]) })),
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{:?}", reply.body);
    Account {
        token: reply.access_token(),
        id: reply.json()["account"]["id"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap(),
    }
}

/// Gives `user` `count` export credits, as a pack purchase would.
pub async fn add_credits(pool: &PgPool, (user, count): (Uuid, i32)) {
    sqlx::query!(
        r#"
        INSERT INTO credit_ledger (id, user_id, delta, reason, paddle_transaction_id)
        VALUES (gen_random_uuid(), $1, $2, 'purchase', 'txn_' || gen_random_uuid())
        "#,
        user,
        count,
    )
    .execute(pool)
    .await
    .unwrap();
}
