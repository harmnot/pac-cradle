//! Sign-in (M1-D3), end to end on a real database: guests, email links that confirm the address,
//! passwords, passkeys, rotating refresh cookies, the back office's passkey-only sign-in and its
//! write log.
// The helpers below are test code too; clippy.toml only exempts `#[test]` functions.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::time::{SystemTime, UNIX_EPOCH};

use axum::http::header::{ACCEPT_ENCODING, CACHE_CONTROL, CONTENT_ENCODING};
use axum::http::{Method, StatusCode};
use common::authenticator::{KeyKind, SoftAuthenticator};
use common::{Reply, TestApi, auth_api, call, link_token};
use paccradle_backend::shared::base64url;
use paccradle_backend::shared::jwt::{self, JwtKey};
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

const CLIENT: &str = "/api/client/v1";
const ADMIN: &str = "/api/admin/v1";
const COOKIE: &str = "paccradle_refresh";
const ADMIN_COOKIE: &str = "paccradle_admin_refresh";
const APP_ORIGIN: &str = "http://localhost:3000";
const ADMIN_ORIGIN: &str = "http://localhost:3001";
const PASSWORD: &str = "a long enough passphrase";

/// A signed-in browser: its access token, refresh cookie and the session body.
struct Browser {
    access: String,
    cookie: String,
    body: Value,
}

impl Browser {
    fn from(reply: &Reply, cookie: &str) -> Self {
        assert!(
            reply.status.is_success(),
            "{}: {:?}",
            reply.status,
            reply.body
        );
        Self {
            access: reply.access_token(),
            cookie: reply.cookie(cookie).expect("a refresh cookie"),
            body: reply.json(),
        }
    }

    fn user_id(&self) -> Uuid {
        self.body["account"]["id"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap()
    }
}

fn client(path: &str) -> String {
    format!("{CLIENT}{path}")
}

fn admin(path: &str) -> String {
    format!("{ADMIN}{path}")
}

async fn guest(api: &TestApi) -> Browser {
    let reply = api.send(call(Method::POST, &client("/auth/guest"))).await;
    assert_eq!(reply.status, StatusCode::CREATED);
    Browser::from(&reply, COOKIE)
}

/// Asks for a link to `email` (from `browser`'s cookie, if any) and returns the email sent.
async fn ask_link(api: &TestApi, (email, browser): (&str, Option<&Browser>)) -> common::EmailSent {
    let mut request =
        call(Method::POST, &client("/auth/email-link")).json(json!({ "email": email }));
    if let Some(browser) = browser {
        request = request.cookie(COOKIE, &browser.cookie);
    }
    let reply = api.send(request).await;
    assert_eq!(reply.status, StatusCode::ACCEPTED, "{:?}", reply.body);
    let mut sent = api.deliver_emails().await;
    assert_eq!(sent.len(), 1);
    sent.remove(0)
}

async fn use_link(api: &TestApi, (token, browser): (&str, Option<&Browser>)) -> Reply {
    let mut request =
        call(Method::POST, &client("/auth/email-link/sign-in")).json(json!({ "token": token }));
    if let Some(browser) = browser {
        request = request.cookie(COOKIE, &browser.cookie);
    }
    api.send(request).await
}

/// A new account for `email`, signed in by its confirmation link.
async fn sign_up(api: &TestApi, email: &str) -> Browser {
    let sent = ask_link(api, (email, None)).await;
    Browser::from(&use_link(api, (&link_token(&sent), None)).await, COOKIE)
}

async fn set_password(api: &TestApi, (browser, body): (&Browser, Value)) -> Reply {
    api.send(
        call(Method::PUT, &client("/me/password"))
            .bearer(&browser.access)
            .json(body),
    )
    .await
}

async fn password_sign_in(api: &TestApi, (email, password): (&str, &str)) -> Reply {
    api.send(
        call(Method::POST, &client("/auth/password/sign-in"))
            .json(json!({ "email": email, "password": password })),
    )
    .await
}

async fn refresh(api: &TestApi, cookie: &str) -> Reply {
    api.send(call(Method::POST, &client("/auth/refresh")).cookie(COOKIE, cookie))
        .await
}

async fn me(api: &TestApi, access: &str) -> Reply {
    api.send(call(Method::GET, &client("/me")).bearer(access))
        .await
}

/// Adds a passkey from `device` to the signed-in account.
async fn add_passkey(
    api: &TestApi,
    (browser, device): (&Browser, &mut SoftAuthenticator),
) -> Reply {
    let options = api
        .send(call(Method::POST, &client("/me/passkeys/options")).bearer(&browser.access))
        .await;
    assert_eq!(options.status, StatusCode::OK, "{:?}", options.body);
    let options = options.json();
    let credential = device.register(&options["public_key"]);
    api.send(
        call(Method::POST, &client("/me/passkeys"))
            .bearer(&browser.access)
            .json(json!({
                "ceremony_id": options["ceremony_id"],
                "credential": credential,
                "name": "  Test laptop ",
            })),
    )
    .await
}

/// Signs in with `device` on the surface under `prefix`.
async fn passkey_sign_in(api: &TestApi, (prefix, device): (&str, &mut SoftAuthenticator)) -> Reply {
    let options = api
        .send(call(
            Method::POST,
            &format!("{prefix}/auth/passkey/sign-in/options"),
        ))
        .await;
    assert_eq!(options.status, StatusCode::OK, "{:?}", options.body);
    let options = options.json();
    let credential = device.sign_in(&options["public_key"]);
    api.send(
        call(Method::POST, &format!("{prefix}/auth/passkey/sign-in"))
            .json(json!({ "ceremony_id": options["ceremony_id"], "credential": credential })),
    )
    .await
}

async fn add_design(pool: &PgPool, user_id: Uuid) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO designs (id, user_id, spec) VALUES ($1, $2, '{}')")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await
        .unwrap();
    id
}

async fn design_owner(pool: &PgPool, design: Uuid) -> Uuid {
    sqlx::query_scalar("SELECT user_id FROM designs WHERE id = $1")
        .bind(design)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test]
async fn a_guest_session_starts_with_an_http_only_cookie_and_reads_its_account(pool: PgPool) {
    let api = auth_api(pool, &[]);
    let reply = api
        .send(call(Method::POST, &client("/auth/guest")).header(ACCEPT_ENCODING, "gzip, br"))
        .await;

    assert_eq!(reply.status, StatusCode::CREATED);
    let cookie = reply.set_cookie(COOKIE);
    assert!(cookie.contains("; Path=/api/client/v1/auth;"), "{cookie}");
    assert!(cookie.contains("; HttpOnly; SameSite=Strict"), "{cookie}");
    assert!(cookie.contains("; Max-Age=2592000;"), "30 days: {cookie}");
    assert!(
        !cookie.contains("Secure"),
        "plain http in development: {cookie}"
    );
    // Token-carrying responses are never stored or compressed (BREACH).
    assert_eq!(reply.header(CACHE_CONTROL.as_str()), "no-store");
    assert_eq!(reply.header(CONTENT_ENCODING.as_str()), "");

    let body = reply.json();
    assert_eq!(body["token_type"], "Bearer");
    assert_eq!(body["expires_in"], 900);
    assert_eq!(body["account"]["role"], "guest");
    assert_eq!(body["account"]["email"], Value::Null);
    assert_eq!(body["account"]["paper_default"], "a4");
    assert_eq!(body["next_steps"], json!([]));

    let account = me(&api, &reply.access_token()).await;
    assert_eq!(account.status, StatusCode::OK);
    assert_eq!(account.json()["id"], body["account"]["id"]);
    assert_eq!(account.header(CACHE_CONTROL.as_str()), "no-store");

    let anonymous = api.send(call(Method::GET, &client("/me"))).await;
    assert_eq!(anonymous.status, StatusCode::UNAUTHORIZED);
    assert_eq!(anonymous.error_code(), "AUTH_REQUIRED");
}

#[sqlx::test]
async fn an_email_link_confirms_the_address_and_turns_the_guest_into_the_account(pool: PgPool) {
    let api = auth_api(pool.clone(), &[]);
    let visitor = guest(&api).await;
    let design = add_design(&pool, visitor.user_id()).await;

    let sent = ask_link(&api, ("Ana@Example.com", Some(&visitor))).await;
    assert_eq!(sent.to, "Ana@Example.com");
    assert_eq!(sent.subject, "Confirm your email for Paccradle");
    assert!(
        sent.text.contains("http://localhost:3000/auth/link#token="),
        "{}",
        sent.text
    );
    assert!(sent.text.contains("expires in 24 hours"), "{}", sent.text);
    // Nothing secret waits in the queue: the job holds only the link's id.
    let payloads: Vec<Value> = sqlx::query_scalar("SELECT payload FROM jobs")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(payloads.len(), 1);
    assert_eq!(
        payloads[0].as_object().unwrap().keys().collect::<Vec<_>>(),
        ["link_id"]
    );

    let token = link_token(&sent);
    let reply = use_link(&api, (&token, Some(&visitor))).await;
    assert_eq!(reply.status, StatusCode::OK, "{:?}", reply.body);
    let ana = Browser::from(&reply, COOKIE);
    assert_eq!(
        ana.user_id(),
        visitor.user_id(),
        "the guest became the account"
    );
    assert_eq!(ana.body["account"]["email"], "Ana@Example.com");
    assert_eq!(ana.body["account"]["role"], "user");
    assert_eq!(
        ana.body["next_steps"],
        json!(["set_password", "add_passkey"])
    );
    assert_ne!(ana.cookie, visitor.cookie, "a new session, not the guest's");
    assert_eq!(design_owner(&pool, design).await, ana.user_id());

    // The guest session ended; the link works once.
    let old = refresh(&api, &visitor.cookie).await;
    assert_eq!(old.error_code(), "AUTH_SESSION_EXPIRED");
    assert!(old.set_cookie(COOKIE).contains("Max-Age=0"), "cleared");
    let again = use_link(&api, (&token, None)).await;
    assert_eq!(again.status, StatusCode::BAD_REQUEST);
    assert_eq!(again.error_code(), "AUTH_LINK_INVALID");
}

#[sqlx::test]
async fn signing_in_from_a_guest_browser_moves_the_guests_work_into_the_account(pool: PgPool) {
    let api = auth_api(pool.clone(), &[]);
    let ana = sign_up(&api, "ana@example.com").await;
    let visitor = guest(&api).await;
    let design = add_design(&pool, visitor.user_id()).await;

    // Another letter case finds the same account: a sign-in link, valid 15 minutes.
    let sent = ask_link(&api, ("ANA@example.com", Some(&visitor))).await;
    assert_eq!(sent.subject, "Your Paccradle sign-in link");
    assert!(sent.text.contains("expires in 15 minutes"), "{}", sent.text);

    let back = Browser::from(
        &use_link(&api, (&link_token(&sent), Some(&visitor))).await,
        COOKIE,
    );
    assert_eq!(back.user_id(), ana.user_id());
    assert_eq!(design_owner(&pool, design).await, ana.user_id());
    let guests: i64 = sqlx::query_scalar("SELECT count(*) FROM users WHERE id = $1")
        .bind(visitor.user_id())
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(guests, 0, "the guest is gone");
}

#[sqlx::test]
async fn email_links_tell_nobody_whether_an_account_exists_and_are_limited_per_address(
    pool: PgPool,
) {
    let api = auth_api(pool, &[]);
    sign_up(&api, "bo@example.com").await;

    let known = api
        .send(
            call(Method::POST, &client("/auth/email-link"))
                .json(json!({ "email": "bo@example.com" })),
        )
        .await;
    let unknown = api
        .send(
            call(Method::POST, &client("/auth/email-link"))
                .json(json!({ "email": "cy@example.com" })),
        )
        .await;
    assert_eq!(known.status, StatusCode::ACCEPTED);
    assert_eq!(unknown.status, StatusCode::ACCEPTED);
    assert_eq!(known.body, unknown.body);

    // bo has had 2 of 3 this window (sign-up and the one above); the 4th is refused.
    let ask = |email: &'static str| {
        call(Method::POST, &client("/auth/email-link")).json(json!({ "email": email }))
    };
    assert_eq!(
        api.send(ask("BO@example.com")).await.status,
        StatusCode::ACCEPTED
    );
    let limited = api.send(ask("bo@example.com")).await;
    assert_eq!(limited.status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(limited.error_code(), "RATE_LIMITED");
    assert_eq!(limited.json()["error"]["meta"]["retry_after_seconds"], 600);

    for bad in [
        "",
        "bo",
        "bo@",
        "@example.com",
        "bo@example",
        "b o@example.com",
        "bo@.com",
    ] {
        let reply = api.send(ask_owned(bad)).await;
        assert_eq!(reply.error_code(), "INVALID_REQUEST", "{bad:?}");
        assert_eq!(reply.json()["error"]["meta"]["field"], "email");
    }
}

fn ask_owned(email: &str) -> common::Call {
    call(Method::POST, &client("/auth/email-link")).json(json!({ "email": email }))
}

#[sqlx::test]
async fn using_one_link_retires_the_others_and_expired_ones_fail(pool: PgPool) {
    let api = auth_api(pool.clone(), &[]);
    let first = ask_link(&api, ("di@example.com", None)).await;
    let second = ask_link(&api, ("di@example.com", None)).await;
    let third = ask_link(&api, ("ed@example.com", None)).await;

    Browser::from(&use_link(&api, (&link_token(&second), None)).await, COOKIE);
    assert_eq!(
        use_link(&api, (&link_token(&first), None))
            .await
            .error_code(),
        "AUTH_LINK_INVALID"
    );

    sqlx::query("UPDATE email_links SET expires_at = now() - interval '1 second'")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        use_link(&api, (&link_token(&third), None))
            .await
            .error_code(),
        "AUTH_LINK_INVALID"
    );
    assert_eq!(
        use_link(&api, ("not-a-token", None)).await.error_code(),
        "AUTH_LINK_INVALID"
    );
}

#[sqlx::test]
async fn a_password_set_after_the_link_signs_in_and_every_mismatch_looks_alike(pool: PgPool) {
    let api = auth_api(pool, &[]);
    let fay = sign_up(&api, "fay@example.com").await;

    let short = set_password(&api, (&fay, json!({ "password": "short" }))).await;
    assert_eq!(short.status, StatusCode::BAD_REQUEST);
    assert_eq!(short.error_code(), "AUTH_PASSWORD_REJECTED");
    assert_eq!(
        short.json()["error"]["meta"],
        json!({ "min_length": 12, "max_length": 128 })
    );
    assert_eq!(
        short.json()["error"]["message"],
        "Choose a password of 12 to 128 characters."
    );

    // Right after the link, no current password is needed.
    let set = set_password(&api, (&fay, json!({ "password": PASSWORD }))).await;
    assert_eq!(set.status, StatusCode::NO_CONTENT, "{:?}", set.body);

    let signed_in = password_sign_in(&api, ("FAY@example.com", PASSWORD)).await;
    assert_eq!(signed_in.status, StatusCode::OK);
    let body = signed_in.json();
    assert_eq!(body["account"]["has_password"], true);
    assert_eq!(body["next_steps"], json!(["add_passkey"]));

    let wrong = password_sign_in(&api, ("fay@example.com", "not the passphrase")).await;
    let unknown = password_sign_in(&api, ("nobody@example.com", PASSWORD)).await;
    let guestless = password_sign_in(&api, ("gil@example.com", PASSWORD)).await;
    for reply in [&wrong, &unknown, &guestless] {
        assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
        assert_eq!(reply.error_code(), "AUTH_INVALID_CREDENTIALS");
        assert_eq!(reply.body, wrong.body);
    }
}

#[sqlx::test]
async fn changing_a_password_later_needs_the_current_one_and_ends_other_sessions(pool: PgPool) {
    let api = auth_api(pool.clone(), &[]);
    let by_link = sign_up(&api, "gus@example.com").await;
    set_password(&api, (&by_link, json!({ "password": PASSWORD }))).await;
    let by_password = Browser::from(
        &password_sign_in(&api, ("gus@example.com", PASSWORD)).await,
        COOKIE,
    );
    sqlx::query("UPDATE sessions SET authenticated_at = now() - interval '1 hour'")
        .execute(&pool)
        .await
        .unwrap();

    // A stale link session, or any password session, must give the current password.
    for browser in [&by_link, &by_password] {
        let reply = set_password(&api, (browser, json!({ "password": "a new passphrase!" }))).await;
        assert_eq!(reply.status, StatusCode::FORBIDDEN);
        assert_eq!(reply.error_code(), "AUTH_REAUTH_REQUIRED");
        assert_eq!(reply.json()["error"]["meta"]["fresh_minutes"], 15);
    }
    let wrong = set_password(
        &api,
        (
            &by_password,
            json!({ "password": "a new passphrase!", "current_password": "nope" }),
        ),
    )
    .await;
    assert_eq!(wrong.error_code(), "AUTH_INVALID_CREDENTIALS");

    let changed = set_password(
        &api,
        (
            &by_password,
            json!({ "password": "a new passphrase!", "current_password": PASSWORD }),
        ),
    )
    .await;
    assert_eq!(changed.status, StatusCode::NO_CONTENT);
    assert_eq!(
        refresh(&api, &by_link.cookie).await.error_code(),
        "AUTH_SESSION_EXPIRED"
    );
    assert_eq!(
        refresh(&api, &by_password.cookie).await.status,
        StatusCode::OK
    );
    assert_eq!(
        password_sign_in(&api, ("gus@example.com", PASSWORD))
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        password_sign_in(&api, ("gus@example.com", "a new passphrase!"))
            .await
            .status,
        StatusCode::OK
    );
}

#[sqlx::test]
async fn guests_need_an_account_for_a_password_or_a_passkey(pool: PgPool) {
    let api = auth_api(pool, &[]);
    let visitor = guest(&api).await;

    let password = set_password(&api, (&visitor, json!({ "password": PASSWORD }))).await;
    assert_eq!(password.status, StatusCode::FORBIDDEN);
    assert_eq!(password.error_code(), "AUTH_ACCOUNT_REQUIRED");
    let passkey = api
        .send(call(Method::POST, &client("/me/passkeys/options")).bearer(&visitor.access))
        .await;
    assert_eq!(passkey.error_code(), "AUTH_ACCOUNT_REQUIRED");
}

#[sqlx::test]
async fn refresh_rotates_the_cookie_and_a_reused_one_ends_the_session(pool: PgPool) {
    let api = auth_api(pool.clone(), &[]);
    let hal = sign_up(&api, "hal@example.com").await;

    let first = refresh(&api, &hal.cookie).await;
    assert_eq!(first.status, StatusCode::OK);
    let rotated = first.cookie(COOKIE).expect("a new cookie");
    assert_ne!(rotated, hal.cookie);
    assert_eq!(first.json()["next_steps"], json!([]));
    assert_eq!(me(&api, &first.access_token()).await.status, StatusCode::OK);

    // Another tab with the old cookie, a moment later: answered, without a cookie (it has
    // the new one already).
    let racing = refresh(&api, &hal.cookie).await;
    assert_eq!(racing.status, StatusCode::OK);
    assert_eq!(racing.set_cookie(COOKIE), "");
    assert_eq!(racing.header(CACHE_CONTROL.as_str()), "no-store");
    assert_eq!(racing.header(CONTENT_ENCODING.as_str()), "");

    // The old cookie, well after: someone else may hold the session, so it ends for both.
    sqlx::query("UPDATE sessions SET rotated_at = now() - interval '1 minute'")
        .execute(&pool)
        .await
        .unwrap();
    let reused = refresh(&api, &hal.cookie).await;
    assert_eq!(reused.status, StatusCode::UNAUTHORIZED);
    assert_eq!(reused.error_code(), "AUTH_SESSION_EXPIRED");
    assert_eq!(
        refresh(&api, &rotated).await.error_code(),
        "AUTH_SESSION_EXPIRED"
    );

    let none = api.send(call(Method::POST, &client("/auth/refresh"))).await;
    assert_eq!(none.error_code(), "AUTH_SESSION_EXPIRED");
}

#[sqlx::test]
async fn sign_out_ends_the_session_and_clears_the_cookie(pool: PgPool) {
    let api = auth_api(pool, &[]);
    let ivy = sign_up(&api, "ivy@example.com").await;

    let out = api
        .send(call(Method::POST, &client("/auth/sign-out")).cookie(COOKIE, &ivy.cookie))
        .await;
    assert_eq!(out.status, StatusCode::NO_CONTENT);
    assert!(
        out.set_cookie(COOKIE)
            .starts_with("paccradle_refresh=; Path=/api/client/v1/auth; Max-Age=0")
    );
    assert_eq!(
        refresh(&api, &ivy.cookie).await.error_code(),
        "AUTH_SESSION_EXPIRED"
    );
    // Without a cookie it still answers 204.
    let again = api
        .send(call(Method::POST, &client("/auth/sign-out")))
        .await;
    assert_eq!(again.status, StatusCode::NO_CONTENT);
}

#[sqlx::test]
async fn access_tokens_work_only_unexpired_untouched_and_on_their_own_surface(pool: PgPool) {
    let api = auth_api(pool, &[("ADMIN_WEB_URL", ADMIN_ORIGIN)]);
    let jo = sign_up(&api, "jo@example.com").await;

    let on_admin = api
        .send(call(Method::GET, &admin("/me")).bearer(&jo.access))
        .await;
    assert_eq!(on_admin.error_code(), "AUTH_REQUIRED");

    let mut tampered = jo.access.clone();
    tampered.pop();
    tampered.push('A');
    assert_eq!(me(&api, &tampered).await.error_code(), "AUTH_REQUIRED");

    let key = JwtKey::new(api.state.config.auth.jwt_secret.expose());
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let claims = |aud: &str, exp: u64| {
        json!({ "sub": jo.user_id(), "sid": Uuid::now_v7(), "aud": aud, "role": "admin",
                "iat": now, "exp": exp })
    };
    let expired = jwt::encode(&key, &claims("client", now - 1)).unwrap();
    assert_eq!(me(&api, &expired).await.error_code(), "AUTH_REQUIRED");
    let valid = jwt::encode(&key, &claims("client", now + 60)).unwrap();
    assert_eq!(me(&api, &valid).await.status, StatusCode::OK);
    // A token naming the admin role, made for the client surface, opens nothing there.
    let admin_role_client_token = api
        .send(call(Method::GET, &admin("/me")).bearer(&valid))
        .await;
    assert_eq!(admin_role_client_token.error_code(), "AUTH_REQUIRED");
}

#[sqlx::test]
async fn sign_in_routes_refuse_other_sites_and_have_their_own_rate_limit(pool: PgPool) {
    let api = auth_api(pool, &[("RATE_LIMIT_AUTH_PER_MINUTE", "2")]);

    let evil = api
        .send(call(Method::POST, &client("/auth/guest")).origin("https://evil.example"))
        .await;
    assert_eq!(evil.status, StatusCode::FORBIDDEN);
    assert_eq!(evil.error_code(), "FORBIDDEN");
    let refresh_evil = api
        .send(call(Method::POST, &client("/auth/refresh")).origin("null"))
        .await;
    assert_eq!(refresh_evil.status, StatusCode::FORBIDDEN);

    // Refused origins never reach the limit; two sign-ins do, and the third is over it.
    let own = api
        .send(call(Method::POST, &client("/auth/guest")).origin(APP_ORIGIN))
        .await;
    assert_eq!(own.status, StatusCode::CREATED);
    let second = api.send(call(Method::POST, &client("/auth/guest"))).await;
    assert_eq!(second.status, StatusCode::CREATED);
    let limited = api.send(call(Method::POST, &client("/auth/guest"))).await;
    assert_eq!(limited.status, StatusCode::TOO_MANY_REQUESTS);
    assert!(!limited.header("retry-after").is_empty());
    // Other routes, refresh included, keep the API-wide limit.
    assert_eq!(
        api.send(call(Method::GET, &client("/version")))
            .await
            .status,
        StatusCode::OK
    );
    assert_eq!(
        api.send(call(Method::POST, &client("/auth/refresh")))
            .await
            .error_code(),
        "AUTH_SESSION_EXPIRED"
    );
}

#[sqlx::test]
async fn password_tries_are_limited_per_address(pool: PgPool) {
    let api = auth_api(pool, &[]);
    for _ in 0..5 {
        let reply = password_sign_in(&api, ("kim@example.com", "wrong guess")).await;
        assert_eq!(reply.error_code(), "AUTH_INVALID_CREDENTIALS");
    }
    let limited = password_sign_in(&api, ("KIM@example.com", "wrong guess")).await;
    assert_eq!(limited.status, StatusCode::TOO_MANY_REQUESTS);
    // Another address isn't held back.
    let other = password_sign_in(&api, ("lee@example.com", "wrong guess")).await;
    assert_eq!(other.error_code(), "AUTH_INVALID_CREDENTIALS");
}

#[sqlx::test]
async fn a_passkey_added_after_sign_up_signs_in_and_cannot_be_replayed(pool: PgPool) {
    let api = auth_api(pool.clone(), &[]);
    let mo = sign_up(&api, "mo@example.com").await;
    let mut device = SoftAuthenticator::new(KeyKind::P256, APP_ORIGIN);

    let options = api
        .send(call(Method::POST, &client("/me/passkeys/options")).bearer(&mo.access))
        .await
        .json();
    let public_key = &options["public_key"];
    assert_eq!(
        public_key["rp"],
        json!({ "id": "localhost", "name": "Paccradle" })
    );
    assert_eq!(
        public_key["user"]["id"],
        base64url::encode(mo.user_id().as_bytes())
    );
    assert_eq!(public_key["user"]["name"], "mo@example.com");
    assert_eq!(public_key["timeout"], 300_000);
    assert_eq!(public_key["attestation"], "none");
    assert_eq!(
        public_key["authenticatorSelection"],
        json!({ "residentKey": "required", "requireResidentKey": true, "userVerification": "required" })
    );
    let algorithms: Vec<i64> = public_key["pubKeyCredParams"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["alg"].as_i64().unwrap())
        .collect();
    assert_eq!(algorithms, [-7, -8, -257]);
    assert_eq!(public_key["excludeCredentials"], json!([]));

    let credential = device.register(public_key);
    let body = json!({ "ceremony_id": options["ceremony_id"], "credential": credential });
    let added = api
        .send(
            call(Method::POST, &client("/me/passkeys"))
                .bearer(&mo.access)
                .json(body.clone()),
        )
        .await;
    assert_eq!(added.status, StatusCode::CREATED, "{:?}", added.body);
    assert_eq!(added.json()["name"], "");
    assert_eq!(added.json()["backed_up"], true);
    let transports: Vec<String> = sqlx::query_scalar("SELECT unnest(transports) FROM passkeys")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(transports, ["hybrid", "internal"], "unknown ones dropped");

    // The ceremony was used up.
    let replayed = api
        .send(
            call(Method::POST, &client("/me/passkeys"))
                .bearer(&mo.access)
                .json(body),
        )
        .await;
    assert_eq!(replayed.error_code(), "AUTH_PASSKEY_REJECTED");
    // The same authenticator again: excluded, and refused if it tries anyway.
    let again = add_passkey(&api, (&mo, &mut device)).await;
    assert_eq!(again.status, StatusCode::CONFLICT);
    assert_eq!(again.error_code(), "AUTH_PASSKEY_ALREADY_REGISTERED");
    assert_eq!(me(&api, &mo.access).await.json()["passkey_count"], 1);

    let options = api
        .send(call(Method::POST, &client("/auth/passkey/sign-in/options")))
        .await
        .json();
    assert_eq!(options["public_key"]["rpId"], "localhost");
    assert_eq!(options["public_key"]["userVerification"], "required");
    assert_eq!(options["public_key"]["allowCredentials"], json!([]));
    let answer = json!({
        "ceremony_id": options["ceremony_id"],
        "credential": device.sign_in(&options["public_key"]),
    });
    let signed_in = api
        .send(call(Method::POST, &client("/auth/passkey/sign-in")).json(answer.clone()))
        .await;
    assert_eq!(signed_in.status, StatusCode::OK, "{:?}", signed_in.body);
    let body = signed_in.json();
    assert_eq!(body["account"]["id"], mo.body["account"]["id"]);
    // No password yet; no passkey prompt after a passkey sign-in.
    assert_eq!(body["next_steps"], json!(["set_password"]));
    assert!(signed_in.cookie(COOKIE).is_some());

    let replayed = api
        .send(call(Method::POST, &client("/auth/passkey/sign-in")).json(answer))
        .await;
    assert_eq!(replayed.status, StatusCode::BAD_REQUEST);
    assert_eq!(replayed.error_code(), "AUTH_PASSKEY_REJECTED");

    // Right after a passkey sign-in, a password can be set without one.
    let fresh = Browser::from(&signed_in, COOKIE);
    let set = set_password(&api, (&fresh, json!({ "password": PASSWORD }))).await;
    assert_eq!(set.status, StatusCode::NO_CONTENT);

    // An unknown passkey, and a known one used from another site, are refused alike.
    let mut stranger = SoftAuthenticator::new(KeyKind::Ed25519, APP_ORIGIN);
    stranger.user_handle = Some(vec![1; 16]);
    let unknown = passkey_sign_in(&api, (CLIENT, &mut stranger)).await;
    assert_eq!(unknown.error_code(), "AUTH_PASSKEY_REJECTED");
    device.origin = "https://evil.example".to_owned();
    let phished = passkey_sign_in(&api, (CLIENT, &mut device)).await;
    assert_eq!(phished.error_code(), "AUTH_PASSKEY_REJECTED");
}

#[sqlx::test]
async fn admins_sign_in_to_the_back_office_with_a_passkey_only(pool: PgPool) {
    let api = auth_api(pool.clone(), &[("ADMIN_WEB_URL", ADMIN_ORIGIN)]);
    let nia = sign_up(&api, "nia@example.com").await;
    let mut device = SoftAuthenticator::new(KeyKind::P256, APP_ORIGIN);
    assert_eq!(
        add_passkey(&api, (&nia, &mut device)).await.status,
        StatusCode::CREATED
    );

    // Not an admin yet: the passkey works on the app, not on the back office.
    device.origin = ADMIN_ORIGIN.to_owned();
    let refused = passkey_sign_in(&api, (ADMIN, &mut device)).await;
    assert_eq!(refused.error_code(), "AUTH_PASSKEY_REJECTED");

    sqlx::query("UPDATE users SET role = 'admin' WHERE id = $1")
        .bind(nia.user_id())
        .execute(&pool)
        .await
        .unwrap();
    // The app's origin can't answer a back-office ceremony.
    device.origin = APP_ORIGIN.to_owned();
    let wrong_origin = passkey_sign_in(&api, (ADMIN, &mut device)).await;
    assert_eq!(wrong_origin.error_code(), "AUTH_PASSKEY_REJECTED");

    device.origin = ADMIN_ORIGIN.to_owned();
    let reply = passkey_sign_in(&api, (ADMIN, &mut device)).await;
    assert_eq!(reply.status, StatusCode::OK, "{:?}", reply.body);
    let cookie = reply.set_cookie(ADMIN_COOKIE);
    assert!(cookie.contains("; Path=/api/admin/v1/auth;"), "{cookie}");
    assert!(cookie.contains("; Max-Age=43200;"), "12 hours: {cookie}");
    let back_office = Browser::from(&reply, ADMIN_COOKIE);
    assert_eq!(back_office.body["expires_in"], 300);
    assert_eq!(back_office.body["account"]["role"], "admin");
    assert_eq!(back_office.body["next_steps"], json!([]));

    let admin_me = api
        .send(call(Method::GET, &admin("/me")).bearer(&back_office.access))
        .await;
    assert_eq!(admin_me.status, StatusCode::OK);
    assert_eq!(admin_me.json()["email"], "nia@example.com");
    assert_eq!(
        api.send(call(Method::GET, &admin("/version")).bearer(&back_office.access))
            .await
            .status,
        StatusCode::OK
    );
    // Back-office tokens are for the back office only; app tokens of admins aren't enough.
    assert_eq!(
        me(&api, &back_office.access).await.error_code(),
        "AUTH_REQUIRED"
    );
    assert_eq!(
        api.send(call(Method::GET, &admin("/me")).bearer(&nia.access))
            .await
            .error_code(),
        "AUTH_REQUIRED"
    );
    // No email link or password route exists on the back office.
    for path in ["/auth/email-link", "/auth/password/sign-in", "/auth/guest"] {
        let reply = api
            .send(call(Method::POST, &admin(path)).json(json!({})))
            .await;
        assert_eq!(reply.error_code(), "AUTH_REQUIRED", "{path}");
    }

    let admin_refresh =
        |cookie: String| call(Method::POST, &admin("/auth/refresh")).cookie(ADMIN_COOKIE, &cookie);
    let refreshed = api.send(admin_refresh(back_office.cookie.clone())).await;
    assert_eq!(refreshed.status, StatusCode::OK);
    let next = refreshed.cookie(ADMIN_COOKIE).unwrap();
    // The session doesn't slide: the cookie lasts until 12 hours after sign-in.
    let max_age: u64 = refreshed
        .set_cookie(ADMIN_COOKIE)
        .split("Max-Age=")
        .nth(1)
        .and_then(|rest| rest.split(';').next())
        .unwrap()
        .parse()
        .unwrap();
    assert!((43_100..=43_200).contains(&max_age), "{max_age}");

    // Taking the role away ends the back-office session at its next refresh.
    sqlx::query("UPDATE users SET role = 'user' WHERE id = $1")
        .bind(nia.user_id())
        .execute(&pool)
        .await
        .unwrap();
    let demoted = api.send(admin_refresh(next)).await;
    assert_eq!(demoted.error_code(), "AUTH_SESSION_EXPIRED");
}

/// An `admin_audit_log` row: email, method, path, route, status, request id.
type AuditRow = (String, String, String, Option<String>, Option<i16>, String);

#[sqlx::test]
async fn admin_writes_are_logged_before_they_run(pool: PgPool) {
    let api = auth_api(pool.clone(), &[("ADMIN_WEB_URL", ADMIN_ORIGIN)]);
    let oz = sign_up(&api, "oz@example.com").await;
    let mut device = SoftAuthenticator::new(KeyKind::Rsa, APP_ORIGIN);
    add_passkey(&api, (&oz, &mut device)).await;
    sqlx::query("UPDATE users SET role = 'admin' WHERE id = $1")
        .bind(oz.user_id())
        .execute(&pool)
        .await
        .unwrap();
    device.origin = ADMIN_ORIGIN.to_owned();
    let back_office = Browser::from(
        &passkey_sign_in(&api, (ADMIN, &mut device)).await,
        ADMIN_COOKIE,
    );

    // Reads aren't logged; writes are, even to a path with no route.
    api.send(call(Method::GET, &admin("/me")).bearer(&back_office.access))
        .await;
    let write = api
        .send(
            call(Method::POST, &admin("/users/42/credits"))
                .bearer(&back_office.access)
                .json(json!({})),
        )
        .await;
    assert_eq!(write.error_code(), "ROUTE_NOT_FOUND");
    // A caller who isn't an admin leaves no entry.
    api.send(call(Method::POST, &admin("/users/42/credits")).bearer(&oz.access))
        .await;

    let rows: Vec<AuditRow> = sqlx::query_as(
        "SELECT admin_email, method, path, route, status, request_id FROM admin_audit_log",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(rows.len(), 1);
    let (email, method, path, route, status, request_id) = &rows[0];
    assert_eq!(email, "oz@example.com");
    assert_eq!(method, "POST");
    assert_eq!(path, "/api/admin/v1/users/42/credits");
    assert_eq!(route, &None);
    assert_eq!(status, &Some(404));
    assert_eq!(request_id, write.header("x-request-id"));

    // The log can't be rewritten or trimmed.
    let rewrite = sqlx::query("UPDATE admin_audit_log SET path = '/elsewhere'")
        .execute(&pool)
        .await;
    assert!(rewrite.is_err());
    let restatus = sqlx::query("UPDATE admin_audit_log SET status = 200, finished_at = now()")
        .execute(&pool)
        .await;
    assert!(restatus.is_err());
    assert!(
        sqlx::query("DELETE FROM admin_audit_log")
            .execute(&pool)
            .await
            .is_err()
    );
    // Deleting the admin keeps the entry, with their email.
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(oz.user_id())
        .execute(&pool)
        .await
        .unwrap();
    let kept: (Option<Uuid>, String) =
        sqlx::query_as("SELECT admin_id, admin_email FROM admin_audit_log")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(kept, (None, "oz@example.com".to_owned()));
}
