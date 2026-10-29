use std::sync::Arc;

use sqlx::PgPool;

use crate::config::AppConfig;
use crate::repository::audit::PgAuditRepository;
use crate::repository::database::{Database, PgDatabase};
use crate::repository::email_link::PgEmailLinkRepository;
use crate::repository::health::PgHealthRepository;
use crate::repository::job::PgJobRepository;
use crate::repository::passkey::PgPasskeyRepository;
use crate::repository::session::PgSessionRepository;
use crate::repository::user::{PgUserRepository, UserRepository};
use crate::service::audit::{AuditService, DefaultAuditService};
use crate::service::health::{DefaultHealthService, HealthService};
use crate::service::identity::{DefaultIdentityService, IdentityProviders, IdentityService};
use crate::service::job::{DefaultJobService, JobService};
use crate::service::passkey::{DefaultPasskeyService, PasskeyProviders, PasskeyService};
use crate::service::session::{DefaultSessionService, SessionService};
use crate::third_party::email::{EmailSender, SmtpEmailSender};

/// What every handler and job can reach: the settings and the services, each behind its trait
/// so tests can swap one.
#[derive(Clone)]
pub struct AppState {
    /// Settings read once at startup.
    pub config: Arc<AppConfig>,
    /// Readiness checks.
    pub health: Arc<dyn HealthService>,
    /// The job queue.
    pub jobs: Arc<dyn JobService>,
    /// Sessions and access tokens.
    pub sessions: Arc<dyn SessionService>,
    /// Web app accounts: guests, email links, passwords.
    pub identity: Arc<dyn IdentityService>,
    /// Passkey sign-in and registration.
    pub passkeys: Arc<dyn PasskeyService>,
    /// The log of admin writes.
    pub audit: Arc<dyn AuditService>,
}

impl AppState {
    /// The services wired to `pool`, sending email over SMTP.
    pub fn new(config: AppConfig, pool: PgPool) -> Self {
        let email = Arc::new(SmtpEmailSender::new(config.email.clone()));
        Self::with_email(config, pool, email)
    }

    /// The services wired to `pool`, sending email through `email` (tests record it).
    pub fn with_email(config: AppConfig, pool: PgPool, email: Arc<dyn EmailSender>) -> Self {
        let config = Arc::new(config);
        let db: Arc<dyn Database> = Arc::new(PgDatabase::new(pool.clone()));
        let users: Arc<dyn UserRepository> = Arc::new(PgUserRepository::new());
        let health = DefaultHealthService::new(Arc::new(PgHealthRepository::new(pool.clone())));
        let jobs: Arc<dyn JobService> =
            Arc::new(DefaultJobService::new(Arc::new(PgJobRepository::new(pool))));
        let sessions: Arc<dyn SessionService> = Arc::new(DefaultSessionService::new(
            Arc::clone(&db),
            (Arc::new(PgSessionRepository::new()), Arc::clone(&users)),
            config.auth.jwt_secret.expose(),
        ));
        let identity = DefaultIdentityService::new(
            Arc::clone(&config),
            IdentityProviders {
                db: Arc::clone(&db),
                users: Arc::clone(&users),
                links: Arc::new(PgEmailLinkRepository::new()),
                sessions: Arc::clone(&sessions),
                jobs: Arc::clone(&jobs),
                email,
            },
        );
        let passkeys = DefaultPasskeyService::new(
            Arc::clone(&config),
            PasskeyProviders {
                db: Arc::clone(&db),
                passkeys: Arc::new(PgPasskeyRepository::new()),
                users,
                sessions: Arc::clone(&sessions),
            },
        );
        let audit = DefaultAuditService::new(db, Arc::new(PgAuditRepository::new()));
        Self {
            config,
            health: Arc::new(health),
            jobs,
            sessions,
            identity: Arc::new(identity),
            passkeys: Arc::new(passkeys),
            audit: Arc::new(audit),
        }
    }
}
