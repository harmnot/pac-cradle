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
    ]);
    assert_eq!(config.web_origin, "https://app.example.com");
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
