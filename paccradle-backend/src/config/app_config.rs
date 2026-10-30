use std::fmt::Display;
use std::net::{IpAddr, Ipv4Addr};
use std::num::NonZeroU32;
use std::ops::RangeInclusive;
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

use axum::http::{HeaderName, HeaderValue};

use super::env_file;
use crate::shared::crypto::random_bytes;
use crate::shared::secret::Secret;

/// The local settings file. Production passes its env file to Docker instead
/// (`deploy/env/api.*.env.example`), and the image never contains a `.env`.
const ENV_FILE: &str = ".env";

/// Where the Next.js dev server runs (`next dev`).
const DEV_WEB_ORIGIN: &str = "http://localhost:3000";

/// The database `scripts/dev-db.sh` starts.
const DEV_DATABASE_URL: &str = "postgres://paccradle:paccradle@127.0.0.1:54329/paccradle";

/// Mailpit, which `scripts/dev-mail.sh` starts (inbox at http://localhost:8025).
const DEV_SMTP_HOST: &str = "127.0.0.1";
const DEV_SMTP_PORT: u16 = 1025;
const DEV_EMAIL_FROM: &str = "Paccradle <no-reply@localhost>";

/// The shortest access-token signing secret accepted: 256 bits.
const MIN_JWT_SECRET_BYTES: usize = 32;

/// Which kind of server this is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppEnv {
    /// A laptop or the development VPS environment: readable logs, `http://` origins allowed.
    Development,
    /// The live site: JSON logs, `https://` origins only, `APP_WEB_URL` required.
    Production,
}

impl FromStr for AppEnv {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "development" => Ok(Self::Development),
            "production" => Ok(Self::Production),
            _ => Err("must be development or production".to_owned()),
        }
    }
}

/// Every setting the API reads, checked once at startup.
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// `APP_ENV`, default `development`.
    pub env: AppEnv,
    /// `APP_HOST`, default `127.0.0.1` (the deploy script sets `0.0.0.0` inside the container).
    pub host: IpAddr,
    /// `APP_PORT`, default 7773.
    pub port: u16,
    /// `APP_WEB_URL`: the web app's origin, the only one the client API answers CORS for.
    /// Required in production; `http://localhost:3000` in development.
    pub web_origin: HeaderValue,
    /// `ADMIN_WEB_URL`: the back office's origin, the only one the admin API answers CORS for.
    /// Unset means no browser origin may call the admin API.
    pub admin_web_origin: Option<HeaderValue>,
    /// `HTTP_BODY_LIMIT_BYTES`, default 1 MiB (Nginx's `client_max_body_size` for JSON).
    /// Routes that take uploads set their own limit (M1-D4).
    pub http_body_limit_bytes: usize,
    /// `HTTP_REQUEST_TIMEOUT_SECONDS`, default 30: a request still running then gets a 503.
    pub request_timeout: Duration,
    /// `RATE_LIMIT_PER_MINUTE`, default 600: requests one client address may make per minute
    /// across the whole API, on top of Nginx's own limit.
    pub rate_limit_per_minute: NonZeroU32,
    /// `RATE_LIMIT_CLIENT_IP_HEADER`: the header holding the real client address. Behind Nginx
    /// it is `x-real-ip`, which Nginx overwrites; unset, the peer address is used.
    pub client_ip_header: Option<HeaderName>,
    /// `RATE_LIMIT_AUTH_PER_MINUTE`, default 20: sign-in requests (guest, email link,
    /// password, passkey) one client address may make per minute, on top of the API's limit.
    pub rate_limit_auth_per_minute: NonZeroU32,
    /// The database.
    pub database: DatabaseConfig,
    /// `JOB_WORKERS`, default 2 (0 to 32): background workers in this process; 0 runs none.
    pub job_workers: usize,
    /// Sign-in: access tokens and passkeys.
    pub auth: AuthConfig,
    /// Outgoing email (sign-in links).
    pub email: EmailConfig,
    /// Object storage for photos (M1-D4). `None` only in development without `STORAGE_*`:
    /// photos are then kept in memory and lost at restart.
    pub storage: Option<StorageConfig>,
    /// Paddle's price ids, for the checkout a paid export offers (M1-D7; created in F1).
    pub prices: PaddlePrices,
    /// `RATE_LIMIT_UPLOADS_PER_HOUR`, default 30: vision runs one user may start per hour
    /// (each upload and each tap counts). One client address may upload
    /// [`UPLOADS_PER_ADDRESS_FACTOR`] times as many photos.
    pub rate_limit_uploads_per_hour: NonZeroU32,
}

/// How many users' worth of uploads one client address gets (an office or a family behind
/// one address).
pub const UPLOADS_PER_ADDRESS_FACTOR: u32 = 4;

/// Paddle price ids (`pri_…`, Paddle > Catalog), one per thing sold (DEC-02). Required in
/// production; unset in development, the checkout a paid export offers has no price to open.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PaddlePrices {
    /// `PADDLE_PRICE_SINGLE_EXPORT`: one design's production files, $10.
    pub single_export: Option<String>,
    /// `PADDLE_PRICE_EXPORT_PACK`: 5 export credits, $35.
    pub export_pack: Option<String>,
    /// `PADDLE_PRICE_PRO_MONTHLY`: Pro, $12 a month.
    pub pro_monthly: Option<String>,
    /// `PADDLE_PRICE_PRO_YEARLY`: Pro, $139 a year.
    pub pro_yearly: Option<String>,
}

/// An S3-compatible bucket: Cloudflare R2 in production (M1-D4).
#[derive(Debug, Clone)]
pub struct StorageConfig {
    /// `STORAGE_ENDPOINT`, e.g. `https://<account id>.r2.cloudflarestorage.com`. `https://`
    /// only in production; development may use `http://` for a local S3 server.
    pub endpoint: String,
    /// `STORAGE_REGION`, default `auto` (R2's only region).
    pub region: String,
    /// `STORAGE_BUCKET`: a private bucket, 3 to 63 lower-case letters, digits and hyphens.
    pub bucket: String,
    /// `STORAGE_ACCESS_KEY_ID`: an R2 API token's access key, with Object Read & Write on
    /// the bucket.
    pub access_key_id: String,
    /// `STORAGE_SECRET_ACCESS_KEY`: its secret.
    pub secret_access_key: Secret,
}

/// Sign-in settings (M1-D3).
#[derive(Debug, Clone)]
pub struct AuthConfig {
    /// `AUTH_JWT_SECRET`: signs access tokens, at least 32 characters. Required in production;
    /// in development a random one is made at each start (signed-in browsers just refresh).
    pub jwt_secret: Secret<Vec<u8>>,
    /// `WEBAUTHN_RP_ID`: the domain passkeys are bound to. Default: the host of `APP_WEB_URL`.
    /// It must be that host or a parent domain of it (and of `ADMIN_WEB_URL`'s host). Never
    /// change it after launch: every saved passkey stops working.
    pub rp_id: String,
    /// `WEBAUTHN_RP_NAME`, default `Paccradle`: the name some browsers show next to a passkey.
    pub rp_name: String,
}

/// How email leaves: SMTP to a provider (Mailpit in development).
#[derive(Debug, Clone)]
pub struct EmailConfig {
    /// `SMTP_HOST`. Required in production; development uses Mailpit on 127.0.0.1.
    pub smtp_host: String,
    /// `SMTP_PORT`. Default 587 for STARTTLS, 465 for TLS, 1025 (Mailpit) without TLS.
    pub smtp_port: u16,
    /// `SMTP_SECURITY`: `starttls` (default once `SMTP_HOST` is set), `tls`, or `none`
    /// (Mailpit's default; refused in production).
    pub smtp_security: SmtpSecurity,
    /// `SMTP_USERNAME` and `SMTP_PASSWORD`: both or neither; both required in production.
    pub smtp_credentials: Option<(String, Secret)>,
    /// `EMAIL_FROM`, e.g. `Paccradle <no-reply@paccradle.com>`. Required in production.
    pub from: String,
}

/// How the SMTP connection is protected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmtpSecurity {
    /// Plain connection upgraded with STARTTLS, which must succeed (port 587).
    StartTls,
    /// TLS from the first byte (port 465).
    Tls,
    /// No encryption: only for a local catcher such as Mailpit.
    None,
}

impl FromStr for SmtpSecurity {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "starttls" => Ok(Self::StartTls),
            "tls" => Ok(Self::Tls),
            "none" => Ok(Self::None),
            _ => Err("must be starttls, tls or none".to_owned()),
        }
    }
}

/// Where the database is and how to use it.
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    /// `DATABASE_URL`, e.g. `postgres://user:password@host:5432/paccradle`. Required in
    /// production; in development, the database `scripts/dev-db.sh` starts.
    pub url: Secret,
    /// `DATABASE_MAX_CONNECTIONS`, default 10 (1 to 100).
    pub max_connections: u32,
    /// `DATABASE_RUN_MIGRATIONS`, default `true`: apply new migrations at startup.
    pub run_migrations: bool,
}

/// A setting that is missing or can't be used.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// A setting production can't run without.
    #[error("{name} is required in production")]
    Missing {
        /// The environment variable.
        name: &'static str,
    },
    /// A setting whose value can't be used.
    #[error("{name} {problem}")]
    Invalid {
        /// The environment variable.
        name: &'static str,
        /// What is wrong, e.g. "must be a number from 1 to 300".
        problem: String,
    },
    /// The local `.env` file exists but can't be read.
    #[error("cannot read {path}: {source}")]
    EnvFile {
        /// The file.
        path: &'static str,
        /// Why it can't be read.
        source: std::io::Error,
    },
}

impl AppConfig {
    /// Reads the settings from the process environment, then from `./.env` for anything the
    /// environment doesn't set.
    pub fn load() -> Result<Self, ConfigError> {
        let file = env_file::read(Path::new(ENV_FILE)).map_err(|source| ConfigError::EnvFile {
            path: ENV_FILE,
            source,
        })?;
        Self::from_lookup(|name| std::env::var(name).ok().or_else(|| file.get(name).cloned()))
    }

    /// Reads the settings through `lookup` (variable name → value). Empty values count as unset.
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let vars = Vars(lookup);
        let env = vars.parsed("APP_ENV", AppEnv::Development)?;
        let web_origin = match vars.origin("APP_WEB_URL", env)? {
            Some(origin) => origin,
            None if env == AppEnv::Production => {
                return Err(ConfigError::Missing {
                    name: "APP_WEB_URL",
                });
            }
            None => HeaderValue::from_static(DEV_WEB_ORIGIN),
        };
        let admin_web_origin = vars.origin("ADMIN_WEB_URL", env)?;
        let rp_id = vars.rp_id(&web_origin, admin_web_origin.as_ref())?;
        Ok(Self {
            env,
            host: vars.parsed("APP_HOST", IpAddr::V4(Ipv4Addr::LOCALHOST))?,
            port: vars.parsed("APP_PORT", 7773)?,
            web_origin,
            admin_web_origin,
            http_body_limit_bytes: vars.bounded(Bounded {
                name: "HTTP_BODY_LIMIT_BYTES",
                default: 1024 * 1024,
                range: 1024..=64 * 1024 * 1024,
            })?,
            request_timeout: Duration::from_secs(vars.bounded(Bounded {
                name: "HTTP_REQUEST_TIMEOUT_SECONDS",
                default: 30,
                range: 1..=300,
            })?),
            // bounded() keeps it at 1 or more, so the fallback never applies.
            rate_limit_per_minute: NonZeroU32::new(vars.bounded(Bounded {
                name: "RATE_LIMIT_PER_MINUTE",
                default: 600,
                range: 1..=100_000,
            })?)
            .unwrap_or(NonZeroU32::MIN),
            client_ip_header: vars.optional("RATE_LIMIT_CLIENT_IP_HEADER")?,
            rate_limit_auth_per_minute: NonZeroU32::new(vars.bounded(Bounded {
                name: "RATE_LIMIT_AUTH_PER_MINUTE",
                default: 20,
                range: 1..=10_000,
            })?)
            .unwrap_or(NonZeroU32::MIN),
            database: DatabaseConfig {
                url: vars.database_url(env)?,
                max_connections: vars.bounded(Bounded {
                    name: "DATABASE_MAX_CONNECTIONS",
                    default: 10,
                    range: 1..=100,
                })?,
                run_migrations: vars.parsed("DATABASE_RUN_MIGRATIONS", true)?,
            },
            job_workers: vars.bounded(Bounded {
                name: "JOB_WORKERS",
                default: 2,
                range: 0..=32,
            })?,
            auth: AuthConfig {
                jwt_secret: vars.jwt_secret(env)?,
                rp_id,
                rp_name: vars.rp_name()?,
            },
            email: vars.email(env)?,
            storage: vars.storage(env)?,
            prices: vars.prices(env)?,
            rate_limit_uploads_per_hour: NonZeroU32::new(vars.bounded(Bounded {
                name: "RATE_LIMIT_UPLOADS_PER_HOUR",
                default: 30,
                range: 1..=10_000,
            })?)
            .unwrap_or(NonZeroU32::MIN),
        })
    }
}

/// A number setting with a default and the values it may take.
struct Bounded<T> {
    name: &'static str,
    default: T,
    range: RangeInclusive<T>,
}

/// Variable lookups that turn values into typed settings and name the variable on failure.
struct Vars<F>(F);

impl<F: Fn(&str) -> Option<String>> Vars<F> {
    fn get(&self, name: &str) -> Option<String> {
        (self.0)(name)
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
    }

    fn optional<T>(&self, name: &'static str) -> Result<Option<T>, ConfigError>
    where
        T: FromStr,
        T::Err: Display,
    {
        let Some(value) = self.get(name) else {
            return Ok(None);
        };
        value
            .parse()
            .map(Some)
            .map_err(|error| ConfigError::Invalid {
                name,
                problem: format!("is not valid: {error}"),
            })
    }

    fn parsed<T>(&self, name: &'static str, default: T) -> Result<T, ConfigError>
    where
        T: FromStr,
        T::Err: Display,
    {
        Ok(self.optional(name)?.unwrap_or(default))
    }

    fn bounded<T: FromStr + PartialOrd + Display>(
        &self,
        setting: Bounded<T>,
    ) -> Result<T, ConfigError> {
        let invalid = || ConfigError::Invalid {
            name: setting.name,
            problem: format!(
                "must be a whole number from {} to {}",
                setting.range.start(),
                setting.range.end()
            ),
        };
        let value = match self.get(setting.name) {
            Some(text) => text.parse().map_err(|_| invalid())?,
            None => setting.default,
        };
        if !setting.range.contains(&value) {
            return Err(invalid());
        }
        Ok(value)
    }

    fn database_url(&self, env: AppEnv) -> Result<Secret, ConfigError> {
        let name = "DATABASE_URL";
        let Some(url) = self.get(name) else {
            return match env {
                AppEnv::Production => Err(ConfigError::Missing { name }),
                AppEnv::Development => Ok(Secret::new(DEV_DATABASE_URL.to_owned())),
            };
        };
        // The value holds a password, so the error never repeats it.
        if !url.starts_with("postgres://") && !url.starts_with("postgresql://") {
            return Err(ConfigError::Invalid {
                name,
                problem: "must start with postgres://".to_owned(),
            });
        }
        Ok(Secret::new(url))
    }

    fn jwt_secret(&self, env: AppEnv) -> Result<Secret<Vec<u8>>, ConfigError> {
        let name = "AUTH_JWT_SECRET";
        let Some(secret) = self.get(name) else {
            return match env {
                AppEnv::Production => Err(ConfigError::Missing { name }),
                AppEnv::Development => random_bytes::<MIN_JWT_SECRET_BYTES>()
                    .map(|bytes| Secret::new(bytes.to_vec()))
                    .map_err(|_| ConfigError::Invalid {
                        name,
                        problem: "is unset and no random one can be made".to_owned(),
                    }),
            };
        };
        if secret.len() < MIN_JWT_SECRET_BYTES {
            return Err(ConfigError::Invalid {
                name,
                problem: format!("must be at least {MIN_JWT_SECRET_BYTES} characters"),
            });
        }
        // The env templates' placeholder would sign tokens anyone can forge.
        if env == AppEnv::Production && secret.contains("CHANGE_ME") {
            return Err(ConfigError::Invalid {
                name,
                problem: "is still the template's placeholder".to_owned(),
            });
        }
        Ok(Secret::new(secret.into_bytes()))
    }

    fn rp_id(
        &self,
        web_origin: &HeaderValue,
        admin_web_origin: Option<&HeaderValue>,
    ) -> Result<String, ConfigError> {
        let name = "WEBAUTHN_RP_ID";
        let app_host = origin_host(web_origin);
        let rp_id = self
            .get(name)
            .map_or_else(|| app_host.clone(), |id| id.to_ascii_lowercase());
        let covers = |host: &str| host == rp_id || host.ends_with(&format!(".{rp_id}"));
        let admin_host = admin_web_origin.map(origin_host);
        if rp_id.is_empty() || !covers(&app_host) || admin_host.is_some_and(|host| !covers(&host)) {
            return Err(ConfigError::Invalid {
                name,
                problem: "must be the host of APP_WEB_URL (and of ADMIN_WEB_URL), or a parent \
                          domain of both, e.g. paccradle.com"
                    .to_owned(),
            });
        }
        Ok(rp_id)
    }

    fn rp_name(&self) -> Result<String, ConfigError> {
        let name = "WEBAUTHN_RP_NAME";
        let rp_name = self.get(name).unwrap_or_else(|| "Paccradle".to_owned());
        if rp_name.chars().count() > 64 {
            return Err(ConfigError::Invalid {
                name,
                problem: "must be 1 to 64 characters".to_owned(),
            });
        }
        Ok(rp_name)
    }

    fn email(&self, env: AppEnv) -> Result<EmailConfig, ConfigError> {
        let production = env == AppEnv::Production;
        let required = |name: &'static str| match production {
            true => Err(ConfigError::Missing { name }),
            false => Ok(None),
        };
        let host = match self.get("SMTP_HOST") {
            Some(host) => Some(host),
            None => required("SMTP_HOST")?,
        };
        let default_security = match host {
            Some(_) => SmtpSecurity::StartTls,
            None => SmtpSecurity::None,
        };
        let smtp_security = self.parsed("SMTP_SECURITY", default_security)?;
        if production && smtp_security == SmtpSecurity::None {
            return Err(ConfigError::Invalid {
                name: "SMTP_SECURITY",
                problem: "must be starttls or tls in production".to_owned(),
            });
        }
        let default_port = match (smtp_security, &host) {
            (SmtpSecurity::StartTls, _) => 587,
            (SmtpSecurity::Tls, _) => 465,
            (SmtpSecurity::None, Some(_)) => 25,
            (SmtpSecurity::None, None) => DEV_SMTP_PORT,
        };
        let smtp_credentials = match (self.get("SMTP_USERNAME"), self.get("SMTP_PASSWORD")) {
            (Some(username), Some(password)) => Some((username, Secret::new(password))),
            (None, None) if !production => None,
            (None, _) => {
                return Err(ConfigError::Missing {
                    name: "SMTP_USERNAME",
                });
            }
            (_, None) => {
                return Err(ConfigError::Missing {
                    name: "SMTP_PASSWORD",
                });
            }
        };
        let from = match self.get("EMAIL_FROM") {
            Some(from) => from,
            None => required("EMAIL_FROM")?.unwrap_or_else(|| DEV_EMAIL_FROM.to_owned()),
        };
        if from.parse::<lettre::message::Mailbox>().is_err() {
            return Err(ConfigError::Invalid {
                name: "EMAIL_FROM",
                problem: "must be an address, e.g. Paccradle <no-reply@paccradle.com>".to_owned(),
            });
        }
        Ok(EmailConfig {
            smtp_host: host.unwrap_or_else(|| DEV_SMTP_HOST.to_owned()),
            smtp_port: self.parsed("SMTP_PORT", default_port)?,
            smtp_security,
            smtp_credentials,
            from,
        })
    }

    fn prices(&self, env: AppEnv) -> Result<PaddlePrices, ConfigError> {
        let price = |name: &'static str| -> Result<Option<String>, ConfigError> {
            let Some(id) = self.get(name) else {
                return match env {
                    AppEnv::Production => Err(ConfigError::Missing { name }),
                    AppEnv::Development => Ok(None),
                };
            };
            let shaped = id.strip_prefix("pri_").is_some_and(|rest| {
                !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_alphanumeric())
            });
            if !shaped {
                return Err(ConfigError::Invalid {
                    name,
                    problem: "must be a Paddle price id, e.g. pri_01h…".to_owned(),
                });
            }
            Ok(Some(id))
        };
        Ok(PaddlePrices {
            single_export: price("PADDLE_PRICE_SINGLE_EXPORT")?,
            export_pack: price("PADDLE_PRICE_EXPORT_PACK")?,
            pro_monthly: price("PADDLE_PRICE_PRO_MONTHLY")?,
            pro_yearly: price("PADDLE_PRICE_PRO_YEARLY")?,
        })
    }

    fn storage(&self, env: AppEnv) -> Result<Option<StorageConfig>, ConfigError> {
        const NAMES: [&str; 4] = [
            "STORAGE_ENDPOINT",
            "STORAGE_BUCKET",
            "STORAGE_ACCESS_KEY_ID",
            "STORAGE_SECRET_ACCESS_KEY",
        ];
        let values = NAMES.map(|name| self.get(name));
        let any_set = values.iter().any(Option::is_some);
        let [
            Some(endpoint),
            Some(bucket),
            Some(access_key_id),
            Some(secret),
        ] = values
        else {
            let name = NAMES
                .iter()
                .zip(&values)
                .find_map(|(name, value)| value.is_none().then_some(*name))
                .unwrap_or(NAMES[0]);
            return match (any_set, env) {
                (false, AppEnv::Development) => Ok(None),
                (false, AppEnv::Production) => Err(ConfigError::Missing { name }),
                (true, _) => Err(ConfigError::Invalid {
                    name,
                    problem: "must be set too: the STORAGE_* settings go together".to_owned(),
                }),
            };
        };
        let endpoint = parse_origin(&endpoint, env)
            .map_err(|problem| ConfigError::Invalid {
                name: "STORAGE_ENDPOINT",
                problem,
            })?
            .to_str()
            .unwrap_or_default()
            .to_owned();
        let region = self
            .get("STORAGE_REGION")
            .unwrap_or_else(|| "auto".to_owned());
        if region.is_empty()
            || !region
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        {
            return Err(ConfigError::Invalid {
                name: "STORAGE_REGION",
                problem: "must be lower-case letters, digits and hyphens, e.g. auto".to_owned(),
            });
        }
        if !valid_bucket(&bucket) {
            return Err(ConfigError::Invalid {
                name: "STORAGE_BUCKET",
                problem: "must be 3 to 63 lower-case letters, digits and hyphens, starting and \
                          ending with a letter or digit"
                    .to_owned(),
            });
        }
        // The key travels in a header: printable, no spaces. The secret is never repeated.
        if !access_key_id.bytes().all(|b| b.is_ascii_graphic()) {
            return Err(ConfigError::Invalid {
                name: "STORAGE_ACCESS_KEY_ID",
                problem: "must be printable ASCII with no spaces".to_owned(),
            });
        }
        if env == AppEnv::Production && secret.contains("CHANGE_ME") {
            return Err(ConfigError::Invalid {
                name: "STORAGE_SECRET_ACCESS_KEY",
                problem: "is still the template's placeholder".to_owned(),
            });
        }
        Ok(Some(StorageConfig {
            endpoint,
            region,
            bucket,
            access_key_id,
            secret_access_key: Secret::new(secret),
        }))
    }

    fn origin(&self, name: &'static str, env: AppEnv) -> Result<Option<HeaderValue>, ConfigError> {
        let Some(value) = self.get(name) else {
            return Ok(None);
        };
        parse_origin(&value, env)
            .map(Some)
            .map_err(|problem| ConfigError::Invalid { name, problem })
    }
}

/// An S3 bucket name as R2 allows it: 3 to 63 lower-case letters, digits and hyphens, starting
/// and ending with a letter or digit.
fn valid_bucket(bucket: &str) -> bool {
    let edges_ok = |b: Option<u8>| b.is_some_and(|b| b.is_ascii_lowercase() || b.is_ascii_digit());
    (3..=63).contains(&bucket.len())
        && bucket
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        && edges_ok(bucket.bytes().next())
        && edges_ok(bucket.bytes().last())
}

/// The host of an origin made by [`parse_origin`]: `https://app.example.com:8443` gives
/// `app.example.com`, `http://[::1]:3000` gives `::1`.
fn origin_host(origin: &HeaderValue) -> String {
    let text = origin.to_str().unwrap_or_default();
    let authority = text.split_once("://").map_or(text, |(_, rest)| rest);
    let host = match authority.strip_prefix('[') {
        Some(bracketed) => bracketed.split(']').next().unwrap_or_default(),
        None => authority.split(':').next().unwrap_or_default(),
    };
    host.to_owned()
}

/// Turns `https://app.example.com` (a trailing `/` is fine) into the `Origin` header value
/// browsers send. Production allows only `https://`.
fn parse_origin(value: &str, env: AppEnv) -> Result<HeaderValue, String> {
    let shape = "must be an origin such as https://app.example.com, with no path";
    let Some((scheme, rest)) = value.split_once("://") else {
        return Err(shape.to_owned());
    };
    match (scheme, env) {
        ("https", _) | ("http", AppEnv::Development) => {}
        ("http", AppEnv::Production) => {
            return Err("must start with https:// in production".to_owned());
        }
        _ => return Err(shape.to_owned()),
    }
    let host = rest.strip_suffix('/').unwrap_or(rest).to_ascii_lowercase();
    let host_ok = !host.is_empty()
        && host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | ':' | '[' | ']'));
    if !host_ok {
        return Err(shape.to_owned());
    }
    HeaderValue::from_str(&format!("{scheme}://{host}")).map_err(|_| shape.to_owned())
}
