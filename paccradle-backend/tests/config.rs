//! Settings (M1-D1): defaults, production rules, and errors that name the variable.
// The helpers below are test code too; clippy.toml only exempts `#[test]` functions.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::collections::HashMap;
use std::time::Duration;

use common::config;
use paccradle_backend::config::{AppConfig, AppEnv, ConfigError, env_file};

fn load(vars: &[(&str, &str)]) -> Result<AppConfig, ConfigError> {
    let vars: HashMap<&str, &str> = vars.iter().copied().collect();
    AppConfig::from_lookup(|name| vars.get(name).map(|value| (*value).to_owned()))
}

fn error(vars: &[(&str, &str)]) -> String {
    load(vars).expect_err("config is refused").to_string()
}

#[test]
fn development_runs_with_no_settings() {
    let config = config(&[]);

    assert_eq!(config.env, AppEnv::Development);
    assert_eq!(config.host.to_string(), "127.0.0.1");
    assert_eq!(config.port, 7773);
    assert_eq!(config.web_origin, "http://localhost:3000");
    assert_eq!(config.admin_web_origin, None);
    assert_eq!(config.http_body_limit_bytes, 1024 * 1024);
    assert_eq!(config.request_timeout, Duration::from_secs(30));
    assert_eq!(config.rate_limit_per_minute.get(), 600);
    assert_eq!(config.client_ip_header, None);
}

#[test]
fn production_needs_an_https_web_app_origin() {
    assert_eq!(
        error(&[("APP_ENV", "production")]),
        "APP_WEB_URL is required in production"
    );
    assert!(
        error(&[
            ("APP_ENV", "production"),
            ("APP_WEB_URL", "http://app.example.com")
        ])
        .contains("https:// in production")
    );
    assert!(
        error(&[
            ("APP_ENV", "production"),
            ("APP_WEB_URL", "https://app.example.com"),
            ("ADMIN_WEB_URL", "http://admin.example.com"),
        ])
        .starts_with("ADMIN_WEB_URL")
    );

    let config = config(&[
        ("APP_ENV", "production"),
        (
            "DATABASE_URL",
            "postgres://api:secret@db.internal:5432/paccradle",
        ),
        ("APP_WEB_URL", "https://App.Example.com/"),
        ("ADMIN_WEB_URL", "https://admin.example.com"),
        ("RATE_LIMIT_CLIENT_IP_HEADER", "X-Real-IP"),
        // Sign-in (M1-D3): tests/auth_primitives.rs checks each of these.
        ("WEBAUTHN_RP_ID", "example.com"),
        ("AUTH_JWT_SECRET", "0123456789abcdef0123456789abcdef"),
        ("SMTP_HOST", "mail.example.com"),
        ("SMTP_USERNAME", "api"),
        ("SMTP_PASSWORD", "secret"),
        ("EMAIL_FROM", "Example <no-reply@example.com>"),
        // Photos (M1-D4): checked below.
        (
            "STORAGE_ENDPOINT",
            "https://account.r2.cloudflarestorage.com",
        ),
        ("STORAGE_BUCKET", "paccradle-photos"),
        ("STORAGE_ACCESS_KEY_ID", "key-id"),
        ("STORAGE_SECRET_ACCESS_KEY", "key-secret"),
        // Checkout prices (M1-D7): checked below.
        ("PADDLE_PRICE_SINGLE_EXPORT", "pri_01single"),
        ("PADDLE_PRICE_EXPORT_PACK", "pri_01pack"),
        ("PADDLE_PRICE_PRO_MONTHLY", "pri_01monthly"),
        ("PADDLE_PRICE_PRO_YEARLY", "pri_01yearly"),
    ]);
    assert_eq!(config.web_origin, "https://app.example.com");
    assert_eq!(config.prices.single_export.as_deref(), Some("pri_01single"));
    assert_eq!(
        config
            .admin_web_origin
            .as_ref()
            .map(|o| o.to_str().unwrap()),
        Some("https://admin.example.com")
    );
    assert_eq!(
        config.client_ip_header.as_ref().map(|h| h.as_str()),
        Some("x-real-ip")
    );
}

#[test]
fn origins_must_have_no_path() {
    for value in [
        "https://app.example.com/pay",
        "app.example.com",
        "ftp://x.com",
        "https://",
    ] {
        let message = error(&[("APP_WEB_URL", value)]);
        assert!(
            message.starts_with("APP_WEB_URL must"),
            "{value}: {message}"
        );
    }
}

#[test]
fn bad_values_name_their_variable() {
    assert!(error(&[("APP_ENV", "staging")]).starts_with("APP_ENV is not valid"));
    assert!(error(&[("APP_PORT", "seventy")]).starts_with("APP_PORT is not valid"));
    assert!(error(&[("APP_HOST", "localhost")]).starts_with("APP_HOST is not valid"));
    assert_eq!(
        error(&[("HTTP_REQUEST_TIMEOUT_SECONDS", "0")]),
        "HTTP_REQUEST_TIMEOUT_SECONDS must be a whole number from 1 to 300"
    );
    assert!(error(&[("HTTP_BODY_LIMIT_BYTES", "10")]).contains("from 1024 to"));
    assert!(error(&[("RATE_LIMIT_PER_MINUTE", "0")]).contains("from 1 to 100000"));
    assert!(
        error(&[("RATE_LIMIT_CLIENT_IP_HEADER", "bad header")])
            .starts_with("RATE_LIMIT_CLIENT_IP_HEADER")
    );
}

#[test]
fn empty_values_count_as_unset() {
    let config = config(&[("APP_PORT", ""), ("ADMIN_WEB_URL", "  ")]);

    assert_eq!(config.port, 7773);
    assert_eq!(config.admin_web_origin, None);
}

#[test]
fn env_files_follow_docker_rules() {
    let vars = env_file::parse(
        "# comment\n\
         \n\
         APP_PORT=8080\n\
         APP_HOST = 0.0.0.0 \n\
         EMAIL_FROM=\"Paccradle <hi@example.com>\"\n\
         NO_EQUALS_SIGN\n\
         EMPTY=\n\
         URL=https://example.com/a=b\n",
    );

    assert_eq!(vars["APP_PORT"], "8080");
    assert_eq!(vars["APP_HOST"], "0.0.0.0");
    // Docker keeps quotes, so the file does too (the deploy refuses quoted values).
    assert_eq!(vars["EMAIL_FROM"], "\"Paccradle <hi@example.com>\"");
    assert_eq!(vars["EMPTY"], "");
    assert_eq!(vars["URL"], "https://example.com/a=b");
    assert!(!vars.contains_key("NO_EQUALS_SIGN"));
    assert_eq!(vars.len(), 5);
}

#[test]
fn a_missing_env_file_gives_no_values() {
    let vars = env_file::read(std::path::Path::new("no/such/.env")).unwrap();
    assert!(vars.is_empty());
}

#[test]
fn the_database_defaults_to_the_dev_container_and_production_must_name_one() {
    let config = config(&[]);
    assert_eq!(
        config.database.url.expose(),
        "postgres://paccradle:paccradle@127.0.0.1:54329/paccradle"
    );
    assert_eq!(config.database.max_connections, 10);
    assert!(config.database.run_migrations);
    assert_eq!(config.job_workers, 2);

    assert_eq!(
        error(&[
            ("APP_ENV", "production"),
            ("APP_WEB_URL", "https://app.example.com"),
        ]),
        "DATABASE_URL is required in production"
    );
}

#[test]
fn database_settings_are_checked_without_repeating_the_url() {
    let message = error(&[("DATABASE_URL", "mysql://root:hunter2@db/paccradle")]);
    assert_eq!(message, "DATABASE_URL must start with postgres://");
    assert!(!message.contains("hunter2"));

    assert!(error(&[("DATABASE_MAX_CONNECTIONS", "0")]).contains("from 1 to 100"));
    assert!(
        error(&[("DATABASE_RUN_MIGRATIONS", "yes")])
            .starts_with("DATABASE_RUN_MIGRATIONS is not valid")
    );
    assert!(error(&[("JOB_WORKERS", "33")]).contains("from 0 to 32"));

    let config = config(&[
        ("DATABASE_URL", "postgresql://api:hunter2@db:5432/paccradle"),
        ("DATABASE_RUN_MIGRATIONS", "false"),
        ("JOB_WORKERS", "0"),
    ]);
    assert!(!config.database.run_migrations);
    assert_eq!(config.job_workers, 0);
    // Printing the config (a panic message, a debug log) never shows the password.
    assert!(!format!("{config:?}").contains("hunter2"));
}

#[test]
fn photo_storage_is_optional_in_development_and_needed_whole_in_production() {
    let config = config(&[]);
    assert!(config.storage.is_none(), "photos kept in memory");
    assert_eq!(config.rate_limit_uploads_per_hour.get(), 30);

    let storage = [
        (
            "STORAGE_ENDPOINT",
            "https://Account.r2.cloudflarestorage.com/",
        ),
        ("STORAGE_BUCKET", "paccradle-photos"),
        ("STORAGE_ACCESS_KEY_ID", "key-id"),
        ("STORAGE_SECRET_ACCESS_KEY", "key-secret"),
    ];
    let config = load(&storage).unwrap();
    let stored = config.storage.unwrap();
    assert_eq!(stored.endpoint, "https://account.r2.cloudflarestorage.com");
    assert_eq!(stored.region, "auto");
    assert_eq!(stored.bucket, "paccradle-photos");
    assert_eq!(format!("{:?}", stored.secret_access_key), "[redacted]");

    // They go together, in development too.
    assert_eq!(
        error(&storage[1..]),
        "STORAGE_ENDPOINT must be set too: the STORAGE_* settings go together"
    );
    assert!(error(&storage[..3]).starts_with("STORAGE_SECRET_ACCESS_KEY must be set too"));
    assert_eq!(
        error(&[
            ("APP_ENV", "production"),
            ("APP_WEB_URL", "https://app.example.com"),
            ("DATABASE_URL", "postgres://api:secret@db:5432/paccradle"),
            ("AUTH_JWT_SECRET", "0123456789abcdef0123456789abcdef"),
            ("SMTP_HOST", "mail.example.com"),
            ("SMTP_USERNAME", "api"),
            ("SMTP_PASSWORD", "secret"),
            ("EMAIL_FROM", "Example <no-reply@example.com>"),
        ]),
        "STORAGE_ENDPOINT is required in production"
    );
}

#[test]
fn photo_storage_settings_are_checked() {
    let with = |name: &'static str, value: &'static str| {
        let mut vars = vec![
            (
                "STORAGE_ENDPOINT",
                "https://account.r2.cloudflarestorage.com",
            ),
            ("STORAGE_BUCKET", "paccradle-photos"),
            ("STORAGE_ACCESS_KEY_ID", "key-id"),
            ("STORAGE_SECRET_ACCESS_KEY", "key-secret"),
        ];
        vars.retain(|(var, _)| *var != name);
        vars.push((name, value));
        vars
    };
    // A local S3 server over http is fine in development only, and never with a path.
    assert!(load(&with("STORAGE_ENDPOINT", "http://127.0.0.1:9000")).is_ok());
    assert!(
        error(&with(
            "STORAGE_ENDPOINT",
            "https://account.r2.cloudflarestorage.com/bucket"
        ))
        .starts_with("STORAGE_ENDPOINT must be an origin")
    );
    for bucket in ["ab", "Photos", "-photos", "photos-", "pho_tos"] {
        assert!(
            error(&with("STORAGE_BUCKET", bucket)).starts_with("STORAGE_BUCKET must be 3 to 63"),
            "{bucket}"
        );
    }
    assert!(error(&with("STORAGE_REGION", "EU West")).starts_with("STORAGE_REGION must be"));
    assert!(
        error(&with("STORAGE_ACCESS_KEY_ID", "key id")).starts_with("STORAGE_ACCESS_KEY_ID must")
    );
    assert!(error(&[("RATE_LIMIT_UPLOADS_PER_HOUR", "0")]).contains("from 1 to 10000"));
}

#[test]
fn checkout_prices_are_optional_in_development_and_shaped_like_paddles() {
    let config = config(&[]);
    assert_eq!(
        config.prices,
        paccradle_backend::config::PaddlePrices::default()
    );
    assert_eq!(
        error(&[("PADDLE_PRICE_EXPORT_PACK", "price_123")]),
        "PADDLE_PRICE_EXPORT_PACK must be a Paddle price id, e.g. pri_01h…"
    );
    let production = [
        ("APP_ENV", "production"),
        ("APP_WEB_URL", "https://app.example.com"),
        ("DATABASE_URL", "postgres://api:secret@db:5432/paccradle"),
        ("AUTH_JWT_SECRET", "0123456789abcdef0123456789abcdef"),
        ("SMTP_HOST", "mail.example.com"),
        ("SMTP_USERNAME", "api"),
        ("SMTP_PASSWORD", "secret"),
        ("EMAIL_FROM", "Example <no-reply@example.com>"),
        (
            "STORAGE_ENDPOINT",
            "https://account.r2.cloudflarestorage.com",
        ),
        ("STORAGE_BUCKET", "paccradle-photos"),
        ("STORAGE_ACCESS_KEY_ID", "key-id"),
        ("STORAGE_SECRET_ACCESS_KEY", "key-secret"),
    ];
    assert_eq!(
        error(&production),
        "PADDLE_PRICE_SINGLE_EXPORT is required in production"
    );
}
