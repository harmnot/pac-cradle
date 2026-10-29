use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::{Map, Value};
use time::OffsetDateTime;
use tokio::sync::Semaphore;
use uuid::Uuid;

use super::{EmailLinkRequest, IdentityService, PasswordSignIn, SetPassword};
use crate::config::AppConfig;
use crate::error::{AppError, ErrorCode};
use crate::locales::{Locale, fill};
use crate::models::email_link::{
    EmailLink, EmailLinkPurpose, NewEmailLink, SendEmailLink, SendEmailLinkPayload,
};
use crate::models::job::NewJob;
use crate::models::session::{AuthUser, SignInMethod, SignedIn};
use crate::models::user::{NewUser, User};
use crate::repository::database::Database;
use crate::repository::email_link::EmailLinkRepository;
use crate::repository::user::UserRepository;
use crate::service::job::JobService;
use crate::service::session::{SessionService, StartSession};
use crate::shared::api_surface::ApiSurface;
use crate::shared::crypto::{NewToken, token_hash};
use crate::shared::future::BoxFuture;
use crate::shared::id::new_id;
use crate::shared::password;
use crate::shared::rate_limiter::RateLimiter;
use crate::third_party::email::{EmailMessage, EmailSender};

/// How long a sign-in link works.
pub const SIGN_IN_LINK_TTL: Duration = Duration::from_secs(15 * 60);
/// How long a sign-up (confirm your email) link works.
pub const SIGN_UP_LINK_TTL: Duration = Duration::from_secs(24 * 60 * 60);
/// Links one address may be sent per [`EMAIL_LINK_WINDOW`], so nobody can flood an inbox.
pub const EMAIL_LINKS_PER_WINDOW: i64 = 3;
/// See [`EMAIL_LINKS_PER_WINDOW`].
pub const EMAIL_LINK_WINDOW: Duration = Duration::from_secs(10 * 60);
/// A password may be changed without the current one this long after a sign-in by email link
/// or passkey (how "forgot password" works).
pub const FRESH_SIGN_IN: Duration = Duration::from_secs(15 * 60);
/// The shortest password accepted. Passwords back up passkeys, so they guard the account
/// alone: long, with no composition rules (NIST SP 800-63B).
pub const MIN_PASSWORD_CHARS: usize = 12;
/// The longest password accepted (Argon2 takes any length; this bounds the work).
pub const MAX_PASSWORD_CHARS: usize = 128;
/// Password sign-ins one address may try per minute, on top of the per-address-IP limit.
const PASSWORD_TRIES_PER_MINUTE: u32 = 5;
/// Password hashes computed at once (each takes 19 MiB and a CPU for tens of milliseconds).
const CONCURRENT_HASHES: usize = 4;
/// Links asked for this long ago are deleted at the next request.
const LINK_RETENTION: Duration = Duration::from_secs(7 * 24 * 60 * 60);

/// What [`DefaultIdentityService`] works with.
pub struct IdentityProviders {
    /// Transactions.
    pub db: Arc<dyn Database>,
    /// `users`.
    pub users: Arc<dyn UserRepository>,
    /// `email_links`.
    pub links: Arc<dyn EmailLinkRepository>,
    /// Sessions and tokens.
    pub sessions: Arc<dyn SessionService>,
    /// The job queue (link emails go out through it).
    pub jobs: Arc<dyn JobService>,
    /// The email provider.
    pub email: Arc<dyn EmailSender>,
}

/// [`IdentityService`] on Postgres, the job queue and an email provider.
pub struct DefaultIdentityService {
    config: Arc<AppConfig>,
    parts: IdentityProviders,
    password_tries: RateLimiter<String>,
    hashing: Semaphore,
}

impl DefaultIdentityService {
    /// A service with `config` (the web app's origin for links) on `parts`.
    pub fn new(config: Arc<AppConfig>, parts: IdentityProviders) -> Self {
        Self {
            config,
            parts,
            password_tries: RateLimiter::new(
                NonZeroU32::new(PASSWORD_TRIES_PER_MINUTE).unwrap_or(NonZeroU32::MIN),
            ),
            hashing: Semaphore::new(CONCURRENT_HASHES),
        }
    }

    /// Runs password hashing off the async threads, a few at a time.
    async fn hash_work<T: Send + 'static>(
        &self,
        work: impl FnOnce() -> T + Send + 'static,
    ) -> Result<T, AppError> {
        let _permit = self.hashing.acquire().await.map_err(internal)?;
        tokio::task::spawn_blocking(work).await.map_err(internal)
    }

    async fn password_matches(
        &self,
        (password, hash): (&str, Option<String>),
    ) -> Result<bool, AppError> {
        let password = password.to_owned();
        // An account without a password costs the same time as one with a wrong password.
        let known = hash.is_some();
        let hash = hash.unwrap_or_else(|| password::DUMMY_HASH.to_owned());
        let matches = self
            .hash_work(move || password::verify(&password, &hash))
            .await?;
        Ok(known && matches)
    }

    fn link_email(&self, link: &EmailLink, token: &str) -> EmailMessage {
        let template = Locale::from_tag(&link.locale).email_link(link.purpose);
        let valid_for = match link.purpose {
            EmailLinkPurpose::SignIn => {
                format!("{} {}", SIGN_IN_LINK_TTL.as_secs() / 60, template.minutes)
            }
            EmailLinkPurpose::SignUp => {
                format!("{} {}", SIGN_UP_LINK_TTL.as_secs() / 3600, template.hours)
            }
        };
        let origin = self.config.web_origin.to_str().unwrap_or_default();
        let mut values = Map::new();
        // The token rides in the fragment, which browsers never send to a server or a Referer.
        values.insert(
            "link".to_owned(),
            Value::String(format!("{origin}/auth/link#token={token}")),
        );
        values.insert("valid_for".to_owned(), Value::String(valid_for));
        EmailMessage {
            to: link.email.clone(),
            subject: template.subject.to_owned(),
            text: fill(template.body, &values),
        }
    }
}

impl IdentityService for DefaultIdentityService {
    fn start_guest(&self, locale: Locale) -> BoxFuture<'_, Result<SignedIn, AppError>> {
        Box::pin(async move {
            let mut tx = self.parts.db.begin().await?;
            let guest = NewUser {
                id: new_id(),
                email: None,
                locale: locale.tag().to_owned(),
            };
            self.parts.users.insert(&mut tx, &guest).await?;
            let start = StartSession {
                user_id: guest.id,
                surface: ApiSurface::Client,
                method: SignInMethod::Guest,
                replacing: None,
            };
            let signed_in = self.parts.sessions.start(&mut tx, start).await?;
            tx.commit().await?;
            Ok(signed_in)
        })
    }

    fn request_email_link<'a>(
        &'a self,
        request: EmailLinkRequest<'a>,
    ) -> BoxFuture<'a, Result<(), AppError>> {
        Box::pin(async move {
            let mut tx = self.parts.db.begin().await?;
            let recent = self
                .parts
                .links
                .count_recent(&mut tx, (request.email, EMAIL_LINK_WINDOW))
                .await?;
            if recent >= EMAIL_LINKS_PER_WINDOW {
                return Err(AppError::new(ErrorCode::RateLimited)
                    .with_meta("retry_after_seconds", EMAIL_LINK_WINDOW.as_secs()));
            }
            let account = self
                .parts
                .users
                .find_by_email(&mut tx, request.email)
                .await?;
            let link = NewEmailLink {
                id: new_id(),
                email: request.email.to_owned(),
                purpose: match account {
                    Some(_) => EmailLinkPurpose::SignIn,
                    None => EmailLinkPurpose::SignUp,
                },
                locale: request.locale.tag().to_owned(),
            };
            self.parts.links.insert(&mut tx, &link).await?;
            let job = NewJob::of::<SendEmailLink>(&SendEmailLinkPayload { link_id: link.id })?;
            self.parts.jobs.enqueue(&mut tx, &job).await?;
            self.parts
                .links
                .delete_older_than(&mut tx, LINK_RETENTION)
                .await?;
            tx.commit().await?;
            Ok(())
        })
    }

    fn deliver_email_link(&self, link_id: Uuid) -> BoxFuture<'_, Result<(), AppError>> {
        Box::pin(async move {
            let mut conn = self.parts.db.connection().await?;
            let Some(link) = self.parts.links.find(&mut conn, link_id).await? else {
                return Ok(());
            };
            if link.used {
                return Ok(());
            }
            let lifetime = match link.purpose {
                EmailLinkPurpose::SignIn => SIGN_IN_LINK_TTL,
                EmailLinkPurpose::SignUp => SIGN_UP_LINK_TTL,
            };
            let token = NewToken::generate()?;
            let stored = self
                .parts
                .links
                .set_token(&mut conn, (link.id, &token.hash, lifetime))
                .await?;
            // Dropped before the send, so a slow provider doesn't hold a database connection.
            drop(conn);
            if !stored {
                return Ok(());
            }
            self.parts
                .email
                .send(&self.link_email(&link, &token.token))
                .await
        })
    }

    fn sign_in_with_email_link<'a>(
        &'a self,
        (token, replacing): (&'a str, Option<&'a str>),
    ) -> BoxFuture<'a, Result<SignedIn, AppError>> {
        Box::pin(async move {
            let mut tx = self.parts.db.begin().await?;
            let link = self
                .parts
                .links
                .consume(&mut tx, &token_hash(token))
                .await?
                .ok_or_else(|| AppError::new(ErrorCode::AuthLinkInvalid))?;
            let users = &self.parts.users;
            users.lock_email(&mut tx, &link.email).await?;
            let user_id = match users.find_by_email(&mut tx, &link.email).await? {
                Some(account) => account.id,
                None => {
                    // A new account: the guest this browser was using becomes it, keeping its
                    // captures and designs; otherwise a fresh row.
                    let guest = match replacing {
                        Some(token) => {
                            self.parts
                                .sessions
                                .current_guest(&mut tx, (ApiSurface::Client, token))
                                .await?
                        }
                        None => None,
                    };
                    let converted = match guest {
                        Some(guest) => {
                            users
                                .confirm_guest_email(&mut tx, (guest, &link.email))
                                .await?
                        }
                        None => false,
                    };
                    match (guest, converted) {
                        (Some(guest), true) => guest,
                        _ => {
                            let user = NewUser {
                                id: new_id(),
                                email: Some(link.email.clone()),
                                locale: link.locale.clone(),
                            };
                            users.insert(&mut tx, &user).await?;
                            user.id
                        }
                    }
                }
            };
            self.parts.links.retire_unused(&mut tx, &link.email).await?;
            let start = StartSession {
                user_id,
                surface: ApiSurface::Client,
                method: SignInMethod::EmailLink,
                replacing,
            };
            let signed_in = self.parts.sessions.start(&mut tx, start).await?;
            tx.commit().await?;
            Ok(signed_in)
        })
    }

    fn sign_in_with_password<'a>(
        &'a self,
        sign_in: PasswordSignIn<'a>,
    ) -> BoxFuture<'a, Result<SignedIn, AppError>> {
        Box::pin(async move {
            let key = sign_in.email.to_lowercase();
            if let Err(wait) = self.password_tries.check(key, Instant::now()) {
                return Err(AppError::new(ErrorCode::RateLimited)
                    .with_meta("retry_after_seconds", wait.as_secs().max(1)));
            }
            let mut conn = self.parts.db.connection().await?;
            let user = self
                .parts
                .users
                .find_by_email(&mut conn, sign_in.email)
                .await?;
            let hash = match &user {
                Some(user) => self.parts.users.password_hash(&mut conn, user.id).await?,
                None => None,
            };
            drop(conn);
            let matches = self.password_matches((sign_in.password, hash)).await?;
            let Some(user) = user.filter(|_| matches) else {
                return Err(AppError::new(ErrorCode::AuthInvalidCredentials));
            };
            let mut tx = self.parts.db.begin().await?;
            let start = StartSession {
                user_id: user.id,
                surface: ApiSurface::Client,
                method: SignInMethod::Password,
                replacing: sign_in.replacing,
            };
            let signed_in = self.parts.sessions.start(&mut tx, start).await?;
            tx.commit().await?;
            Ok(signed_in)
        })
    }

    fn account<'a>(&'a self, user: &'a AuthUser) -> BoxFuture<'a, Result<User, AppError>> {
        Box::pin(async move {
            let mut conn = self.parts.db.connection().await?;
            self.parts
                .users
                .find(&mut conn, user.user_id)
                .await?
                .ok_or_else(|| AppError::new(ErrorCode::AuthRequired))
        })
    }

    fn set_password<'a>(
        &'a self,
        (auth, change): (&'a AuthUser, SetPassword<'a>),
    ) -> BoxFuture<'a, Result<(), AppError>> {
        Box::pin(async move {
            let user = self.account(auth).await?;
            if user.is_guest() {
                return Err(AppError::new(ErrorCode::AuthAccountRequired));
            }
            let length = change.password.chars().count();
            if !(MIN_PASSWORD_CHARS..=MAX_PASSWORD_CHARS).contains(&length) {
                return Err(AppError::new(ErrorCode::AuthPasswordRejected)
                    .with_meta("min_length", MIN_PASSWORD_CHARS)
                    .with_meta("max_length", MAX_PASSWORD_CHARS));
            }
            match change.current_password {
                Some(current) => {
                    let mut conn = self.parts.db.connection().await?;
                    let hash = self.parts.users.password_hash(&mut conn, user.id).await?;
                    drop(conn);
                    if !self.password_matches((current, hash)).await? {
                        return Err(AppError::new(ErrorCode::AuthInvalidCredentials));
                    }
                }
                None => {
                    let session = self.parts.sessions.find(auth.session_id).await?;
                    if !session.is_some_and(|session| is_fresh(&session)) {
                        return Err(AppError::new(ErrorCode::AuthReauthRequired)
                            .with_meta("fresh_minutes", FRESH_SIGN_IN.as_secs() / 60));
                    }
                }
            }
            let password = change.password.to_owned();
            let hash = self.hash_work(move || password::hash(&password)).await??;
            let mut tx = self.parts.db.begin().await?;
            let set = self
                .parts
                .users
                .set_password_hash(&mut tx, (user.id, &hash))
                .await?;
            if !set {
                return Err(AppError::new(ErrorCode::AuthAccountRequired));
            }
            let ended = self
                .parts
                .sessions
                .end_others(&mut tx, (user.id, auth.session_id))
                .await?;
            tx.commit().await?;
            tracing::info!(user = %user.id, other_sessions_ended = ended, "password set");
            Ok(())
        })
    }
}

/// Whether a session proved who the user is recently enough to skip the current password.
fn is_fresh(session: &crate::models::session::Session) -> bool {
    let strong = matches!(
        session.method,
        SignInMethod::EmailLink | SignInMethod::Passkey
    );
    strong
        && !session.revoked
        && OffsetDateTime::now_utc() - session.authenticated_at < FRESH_SIGN_IN
}

fn internal(error: impl std::error::Error + Send + Sync + 'static) -> AppError {
    AppError::new(ErrorCode::Internal).with_source(error)
}
