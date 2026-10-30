//! Development seed data (M1-D9): the same four accounts every time, with the same names,
//! emails and password, so the web app, Postman and manual checks always sign in as the same
//! people. Running it resets those four accounts (and nothing else) to the state below. It
//! refuses to run unless `APP_ENV=development` and the database is on this machine: the
//! password and the admin's passkey key are written here for anyone to read.
//!
//! ```text
//! scripts/dev-seed.sh               # reset the four accounts
//! scripts/dev-seed.sh admin-token   # sign the admin in to the back office with the seeded passkey
//! ```
//!
//! The accounts are listed in `.claude/skills/dev-seed/SKILL.md` (at the repo root) and in
//! `postman/dev-seed-2026-10-30.postman_environment.json` too: change all three together.

#[allow(dead_code, clippy::unwrap_used, clippy::expect_used)]
#[path = "../tests/common/authenticator.rs"]
mod authenticator;
#[allow(dead_code, clippy::unwrap_used, clippy::expect_used)]
#[path = "../tests/common/designs.rs"]
mod designs;

use std::error::Error;
use std::path::Path;
use std::process::ExitCode;
use std::time::Duration;

use authenticator::SoftAuthenticator;
use axum::Router;
use axum::body::Body;
use axum::http::header::{CONTENT_TYPE, COOKIE, SET_COOKIE};
use axum::http::{HeaderMap, Request, StatusCode};
use paccradle_backend::AppState;
use paccradle_backend::config::{AppConfig, AppEnv, env_file};
use paccradle_backend::models::design::NewDesign;
use paccradle_backend::models::export::{ExportFormat, ExportView};
use paccradle_backend::models::session::AuthUser;
use paccradle_backend::models::user::AccountRole;
use paccradle_backend::repository::database;
use paccradle_backend::repository::design::{DesignRepository, PgDesignRepository};
use paccradle_backend::service::export::ExportRequest;
use paccradle_backend::shared::api_surface::ApiSurface;
use paccradle_backend::shared::base64url;
use paccradle_backend::shared::crypto::sha256;
use paccradle_backend::shared::password;
use paccradle_backend::third_party::http::{self, Timeouts};
use serde_json::{Value, json};
use sqlx::{PgConnection, PgPool};
use time::OffsetDateTime;
use tower::ServiceExt;
use uuid::Uuid;

type SeedResult<T> = Result<T, Box<dyn Error>>;

/// Every seeded account's password (12 to 128 characters, as the API asks).
const PASSWORD: &str = "paccradle-dev-pass";

/// One seeded account.
struct Persona {
    id: Uuid,
    name: &'static str,
    email: &'static str,
    role: AccountRole,
    paper: &'static str,
    /// What the account has, for the summary.
    has: &'static str,
}

const ADA: Persona = Persona {
    id: Uuid::from_u128(0x019a0000_5eed_7000_8000_0000000000a1),
    name: "Ada Admin",
    email: "admin@paccradle.test",
    role: AccountRole::Admin,
    paper: "a4",
    has: "the admin role; the back office signs in with the seeded passkey (admin-token)",
};
const BIMA: Persona = Persona {
    id: Uuid::from_u128(0x019a0000_5eed_7000_8000_0000000000b1),
    name: "Bima Buyer",
    email: "buyer@paccradle.test",
    role: AccountRole::User,
    paper: "a4",
    has: "free plan, no credits, 3 designs: SVG/PDF/DXF answer 402 with the checkout",
};
const CITRA: Persona = Persona {
    id: Uuid::from_u128(0x019a0000_5eed_7000_8000_0000000000c1),
    name: "Citra Credits",
    email: "credits@paccradle.test",
    role: AccountRole::User,
    paper: "a4",
    has: "bought a 5-pack and spent 1 on her cradle (unlocked): 4 credits left",
};
const DEWI: Persona = Persona {
    id: Uuid::from_u128(0x019a0000_5eed_7000_8000_0000000000d1),
    name: "Dewi Pro",
    email: "pro@paccradle.test",
    role: AccountRole::User,
    paper: "letter",
    has: "Pro monthly, active: exports without spending credits",
};
const PERSONAS: [&Persona; 4] = [&ADA, &BIMA, &CITRA, &DEWI];

/// One seeded design: its id, owner, name and spec.
struct SeedDesign {
    id: Uuid,
    owner: Uuid,
    name: &'static str,
    spec: fn() -> Value,
}

/// Citra's cradle: the design her credit unlocked.
const CITRA_CRADLE: Uuid = Uuid::from_u128(0x019a0000_5eed_7000_8001_0000000000c1);

const DESIGNS: [SeedDesign; 6] = [
    SeedDesign {
        id: Uuid::from_u128(0x019a0000_5eed_7000_8001_0000000000b1),
        owner: BIMA.id,
        name: "Bima's perfume cradle",
        spec: designs::cradle_spec,
    },
    SeedDesign {
        id: Uuid::from_u128(0x019a0000_5eed_7000_8001_0000000000b2),
        owner: BIMA.id,
        name: "Bima's lipstick card",
        spec: designs::card_spec,
    },
    SeedDesign {
        id: Uuid::from_u128(0x019a0000_5eed_7000_8001_0000000000b3),
        owner: BIMA.id,
        name: "Bima's candle jar grid",
        spec: designs::grid_spec,
    },
    SeedDesign {
        id: CITRA_CRADLE,
        owner: CITRA.id,
        name: "Citra's perfume cradle",
        spec: designs::cradle_spec,
    },
    SeedDesign {
        id: Uuid::from_u128(0x019a0000_5eed_7000_8001_0000000000c2),
        owner: CITRA.id,
        name: "Citra's lipstick card",
        spec: designs::card_spec,
    },
    SeedDesign {
        id: Uuid::from_u128(0x019a0000_5eed_7000_8001_0000000000d1),
        owner: DEWI.id,
        name: "Dewi's candle jar grid",
        spec: designs::grid_spec,
    },
];

/// Ada's seeded passkey: a P-256 key made for this file only (PKCS#8, base64url). It signs in
/// nowhere but a development database this tool seeded.
const ADA_PASSKEY_PKCS8: &str = "MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgal0cPeXAmXhJxQ-VAZwi4hAQDthwuhD5Knmq\
    9ZngY3mhRANCAAQGYhXIpEtPwEGSqHHz4JD0ULURPo5hv_aRwEI8kfQy12-719kpsebe1qyYY2Te8Zf0bDVts8F2a98Si64WV7_N";
const ADA_PASSKEY_ID: Uuid = Uuid::from_u128(0x019a0000_5eed_7000_8002_0000000000a1);
const ADA_PASSKEY_NAME: &str = "Dev seed key (scripts/dev-seed.sh admin-token)";

/// The back office's origin when `ADMIN_WEB_URL` isn't set: a passkey ceremony needs one.
const DEFAULT_ADMIN_WEB_URL: &str = "http://localhost:3001";

/// Database hosts on this machine (`host.docker.internal` from the toolchain container).
const LOCAL_HOSTS: [&str; 4] = ["localhost", "127.0.0.1", "[::1]", "host.docker.internal"];

const ADMIN_COOKIE: &str = "paccradle_admin_refresh";

#[tokio::main]
async fn main() -> ExitCode {
    let command = std::env::args().nth(1).unwrap_or_else(|| "seed".to_owned());
    let result = match command.as_str() {
        "seed" => seed().await,
        "admin-token" => admin_token().await,
        other => Err(format!("unknown command `{other}`: use `seed` or `admin-token`").into()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("dev seed: {error}");
            ExitCode::FAILURE
        }
    }
}

/// The settings from the environment, then `./.env`, as the API reads them, with
/// [`DEFAULT_ADMIN_WEB_URL`] when `ADMIN_WEB_URL` is unset.
fn load_config() -> SeedResult<AppConfig> {
    let file = env_file::read(Path::new(".env"))?;
    let lookup = |name: &str| {
        std::env::var(name)
            .ok()
            .or_else(|| file.get(name).cloned())
            .filter(|value| !value.trim().is_empty())
    };
    let config = AppConfig::from_lookup(|name| match (name, lookup(name)) {
        ("ADMIN_WEB_URL", None) => Some(DEFAULT_ADMIN_WEB_URL.to_owned()),
        (_, value) => value,
    })?;
    Ok(config)
}

/// The host of a `postgres://user:password@host:port/database` URL.
fn database_host(url: &str) -> &str {
    let rest = url.split_once("://").map_or(url, |(_, rest)| rest);
    let authority = rest.split(['/', '?']).next().unwrap_or_default();
    let host_port = authority
        .rsplit_once('@')
        .map_or(authority, |(_, host)| host);
    match host_port.find(']') {
        Some(end) if host_port.starts_with('[') => &host_port[..=end],
        _ => host_port.split(':').next().unwrap_or_default(),
    }
}

/// Refuses anything but a development setup on a database on this machine.
fn refuse_unless_local(config: &AppConfig) -> SeedResult<()> {
    if config.env != AppEnv::Development {
        return Err("refusing to run: APP_ENV isn't development".into());
    }
    let host = database_host(config.database.url.expose());
    if !LOCAL_HOSTS.contains(&host) {
        return Err(
            format!("refusing to run: the database host `{host}` isn't this machine").into(),
        );
    }
    Ok(())
}

/// The app's services on the development database, with its migrations applied.
async fn connect() -> SeedResult<(AppState, PgPool)> {
    let config = load_config()?;
    refuse_unless_local(&config)?;
    let pool = database::connect(&config.database).await?;
    if config.database.run_migrations {
        database::migrate(&pool).await?;
    }
    Ok((AppState::new(config, pool.clone())?, pool))
}

/// Ada's passkey, as her pretend device holds it, for the back office of `config`.
fn ada_device(config: &AppConfig) -> SeedResult<SoftAuthenticator> {
    let origin = config
        .admin_web_origin
        .as_ref()
        .map(|origin| origin.to_str())
        .transpose()?
        .unwrap_or(DEFAULT_ADMIN_WEB_URL);
    let pkcs8 =
        base64url::decode(ADA_PASSKEY_PKCS8).ok_or("the seeded passkey key isn't base64url")?;
    let credential_id = sha256(b"paccradle dev seed: Ada's passkey");
    let mut device = SoftAuthenticator::p256_from_pkcs8((&pkcs8, &credential_id, origin));
    device.rp_id = config.auth.rp_id.clone();
    device.user_handle = Some(ADA.id.as_bytes().to_vec());
    Ok(device)
}

async fn seed() -> SeedResult<()> {
    let (state, pool) = connect().await?;
    let password_hash = password::hash(PASSWORD)?;
    let device = ada_device(&state.config)?;

    let mut tx = pool.begin().await?;
    reset(&mut tx).await?;
    for persona in PERSONAS {
        add_account(&mut tx, (persona, &password_hash)).await?;
    }
    add_ada_passkey(&mut tx, &device).await?;
    for design in &DESIGNS {
        add_design(&mut tx, design).await?;
    }
    add_citras_pack(&mut tx).await?;
    let pro_until = add_dewis_subscription(&mut tx).await?;
    tx.commit().await?;
    let export = spend_citras_credit(&state).await?;

    println!(
        "Seeded {} accounts. Password for every one: {PASSWORD}\n",
        PERSONAS.len()
    );
    for persona in PERSONAS {
        println!(
            "  {:<14} {:<23} {}",
            persona.name, persona.email, persona.has
        );
    }
    println!(
        "\nDewi's Pro runs until {}. Citra's export {} waits for the API's worker \
         (scripts/dev-api.sh) to make its files.",
        pro_until.date(),
        export.export.id,
    );
    println!("Sign in: POST /api/client/v1/auth/password/sign-in {{\"email\", \"password\"}}.");
    println!("Back office: scripts/dev-seed.sh admin-token");
    Ok(())
}

/// Deletes the seeded accounts, and any account using their addresses, with everything they
/// own (the cascade): sessions, passkeys, designs, exports, credits, subscriptions.
async fn reset(conn: &mut PgConnection) -> SeedResult<()> {
    let ids: Vec<Uuid> = PERSONAS.iter().map(|persona| persona.id).collect();
    let emails: Vec<String> = PERSONAS
        .iter()
        .map(|persona| persona.email.to_lowercase())
        .collect();
    sqlx::query!(
        "DELETE FROM users WHERE id = ANY($1) OR lower(email) = ANY($2)",
        &ids,
        &emails,
    )
    .execute(conn)
    .await?;
    Ok(())
}

/// An account with a confirmed email and the seed password.
async fn add_account(
    conn: &mut PgConnection,
    (persona, password_hash): (&Persona, &str),
) -> SeedResult<()> {
    let role = match persona.role {
        AccountRole::Admin => "admin",
        AccountRole::User | AccountRole::Guest => "user",
    };
    sqlx::query!(
        r#"
        INSERT INTO users (id, email, role, email_confirmed_at, password_hash, paper_default)
        VALUES ($1, $2, $3, now(), $4, $5)
        "#,
        persona.id,
        persona.email,
        role,
        password_hash,
        persona.paper,
    )
    .execute(conn)
    .await?;
    Ok(())
}

/// Ada's passkey, as the web app would have registered it from a synced device.
async fn add_ada_passkey(conn: &mut PgConnection, device: &SoftAuthenticator) -> SeedResult<()> {
    sqlx::query!(
        r#"
        INSERT INTO passkeys (id, user_id, credential_id, public_key, algorithm, transports,
                              backup_eligible, backed_up, name)
        VALUES ($1, $2, $3, $4, -7, '{internal}', true, true, $5)
        "#,
        ADA_PASSKEY_ID,
        ADA.id,
        device.credential_id,
        device.cose_key(),
        ADA_PASSKEY_NAME,
    )
    .execute(conn)
    .await?;
    Ok(())
}

/// A design, checked by `insert_core` as the API checks a saved one.
async fn add_design(conn: &mut PgConnection, design: &SeedDesign) -> SeedResult<()> {
    let spec: insert_core::Design = serde_json::from_value((design.spec)())?;
    spec.validate()
        .map_err(|error| format!("{}: {error}", design.name))?;
    let new = NewDesign {
        id: design.id,
        user_id: design.owner,
        name: design.name.to_owned(),
        spec,
    };
    PgDesignRepository::new().insert(conn, &new).await?;
    Ok(())
}

/// Citra's 5-pack, as the Paddle webhook will add it (M1-F4).
async fn add_citras_pack(conn: &mut PgConnection) -> SeedResult<()> {
    sqlx::query!(
        r#"
        INSERT INTO credit_ledger (id, user_id, delta, reason, paddle_transaction_id, note)
        VALUES ($1, $2, 5, 'purchase', 'txn_dev_seed_citra_pack', 'dev seed: a 5-pack')
        "#,
        Uuid::from_u128(0x019a0000_5eed_7000_8003_0000000000c1),
        CITRA.id,
    )
    .execute(conn)
    .await?;
    Ok(())
}

/// Dewi's Pro subscription, active for 30 days from now. Returns when it ends.
async fn add_dewis_subscription(conn: &mut PgConnection) -> SeedResult<OffsetDateTime> {
    let ends = sqlx::query_scalar!(
        r#"
        INSERT INTO subscriptions (id, user_id, paddle_subscription_id, plan, status,
                                   current_period_end, paddle_updated_at)
        VALUES ($1, $2, 'sub_dev_seed_dewi_pro', 'pro_monthly', 'active',
                now() + interval '30 days', now())
        RETURNING current_period_end AS "current_period_end!"
        "#,
        Uuid::from_u128(0x019a0000_5eed_7000_8004_0000000000d1),
        DEWI.id,
    )
    .fetch_one(conn)
    .await?;
    Ok(ends)
}

/// Citra exports her cradle through the export service, as the web app would: one credit
/// spent, the design unlocked, the files' job queued.
async fn spend_citras_credit(state: &AppState) -> SeedResult<ExportView> {
    let citra = AuthUser {
        user_id: CITRA.id,
        session_id: Uuid::nil(),
        surface: ApiSurface::Client,
        role: AccountRole::User,
    };
    let request = ExportRequest {
        user: &citra,
        design_id: CITRA_CRADLE,
        formats: vec![ExportFormat::Svg, ExportFormat::Pdf, ExportFormat::Dxf],
        paper: None,
        version: None,
    };
    Ok(state.exports.request(request).await?)
}

/// A JSON `POST` through the router, as the back office would send it.
async fn post(
    app: &Router,
    (path, body): (&str, Option<Value>),
) -> SeedResult<(StatusCode, HeaderMap, Value)> {
    let request = Request::post(path);
    let request = match body {
        Some(body) => request
            .header(CONTENT_TYPE, "application/json")
            .body(Body::from(serde_json::to_vec(&body)?))?,
        None => request.body(Body::empty())?,
    };
    let response = app.clone().oneshot(request).await?;
    let (parts, body) = response.into_parts();
    let bytes = axum::body::to_bytes(body, usize::MAX).await?;
    let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    Ok((parts.status, parts.headers, json))
}

/// The value of the back-office refresh cookie a response sets.
fn refresh_cookie(headers: &HeaderMap) -> Option<String> {
    headers
        .get_all(SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .find_map(|value| value.strip_prefix(&format!("{ADMIN_COOKIE}=")))
        .and_then(|value| value.split(';').next())
        .map(str::to_owned)
}

/// Where the running development API listens: `DEV_API_URL`, else `APP_PORT` on this machine
/// (`host.docker.internal` from inside the toolchain container).
fn dev_api_url(port: u16) -> String {
    let host = match Path::new("/.dockerenv").exists() {
        true => "host.docker.internal",
        false => "127.0.0.1",
    };
    std::env::var("DEV_API_URL")
        .ok()
        .filter(|url| !url.is_empty())
        .unwrap_or_else(|| format!("http://{host}:{port}"))
}

/// Trades the refresh cookie for a session at the running API, which signs access tokens with
/// its own key (without `AUTH_JWT_SECRET`, each start makes one; the session itself is in the
/// database). Returns the session and the next cookie, or `None` when nothing answers at `url`.
async fn refresh_at((url, cookie): (&str, &str)) -> SeedResult<Option<(Value, String)>> {
    let client = http::client(Timeouts {
        connect: Duration::from_secs(2),
        total: Duration::from_secs(10),
    })?;
    let sent = client
        .post(format!("{url}/api/admin/v1/auth/refresh"))
        .header(COOKIE, format!("{ADMIN_COOKIE}={cookie}"))
        .send()
        .await;
    let response = match sent {
        Ok(response) => response,
        Err(error) if error.is_connect() || error.is_timeout() => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let status = response.status();
    let next_cookie = refresh_cookie(response.headers());
    let session: Value = serde_json::from_slice(&response.bytes().await?)?;
    if !status.is_success() {
        return Err(format!("{url} refused the refresh: {status} {session}").into());
    }
    let next_cookie = next_cookie.ok_or("the refresh set no cookie")?;
    Ok(Some((session, next_cookie)))
}

/// Signs Ada in to the back office with her seeded passkey, through the API's own passkey
/// sign-in, then trades the session's cookie at the running API for an access token it
/// accepts, and prints both.
async fn admin_token() -> SeedResult<()> {
    let (state, _pool) = connect().await?;
    let mut device = ada_device(&state.config)?;
    let port = state.config.port;
    let api = dev_api_url(port);
    let app = paccradle_backend::app(state);

    let (status, _, options) =
        post(&app, ("/api/admin/v1/auth/passkey/sign-in/options", None)).await?;
    if status != StatusCode::OK {
        return Err(format!("the back office gave no passkey options: {status} {options}").into());
    }
    let credential = device.sign_in(&options["public_key"]);
    let answer = json!({ "ceremony_id": options["ceremony_id"], "credential": credential });
    let (status, headers, session) =
        post(&app, ("/api/admin/v1/auth/passkey/sign-in", Some(answer))).await?;
    if status != StatusCode::OK {
        return Err(format!(
            "the back office refused Ada's seeded passkey ({status} {}); run `scripts/dev-seed.sh` first",
            session["error"]["code"]
        )
        .into());
    }
    let cookie = refresh_cookie(&headers).ok_or("the sign-in set no refresh cookie")?;
    println!(
        "{} <{}> is signed in to the back office.\n",
        ADA.name, ADA.email
    );

    let Some((session, cookie)) = refresh_at((&api, &cookie)).await? else {
        println!(
            "No API answers at {api}: start it (scripts/dev-api.sh), then trade this cookie for \
             an access token with POST /api/admin/v1/auth/refresh (Postman: dev-seed, \
             \"Back office: refresh\"), or run this again.\n"
        );
        println!("Refresh cookie, for 12 hours:\n{ADMIN_COOKIE}={cookie}");
        return Ok(());
    };
    let token = session["access_token"].as_str().ok_or("no access token")?;
    println!(
        "Access token from the running API, for {} seconds:\n{token}\n",
        session["expires_in"]
    );
    println!(
        "Refresh cookie, until 12 hours after this sign-in. Each refresh answers with the next \
         one; sending a replaced one again ends the session:\n{ADMIN_COOKIE}={cookie}\n"
    );
    println!(
        "Try it:\n  curl -s http://127.0.0.1:{port}/api/admin/v1/me -H 'Authorization: Bearer {token}'"
    );
    Ok(())
}
