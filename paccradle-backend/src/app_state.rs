use std::sync::Arc;

use sqlx::PgPool;

use crate::config::AppConfig;
use crate::error::AppError;
use crate::repository::audit::PgAuditRepository;
use crate::repository::capture::{CaptureRepository, PgCaptureRepository};
use crate::repository::database::{Database, PgDatabase};
use crate::repository::design::PgDesignRepository;
use crate::repository::email_link::PgEmailLinkRepository;
use crate::repository::entitlement::PgEntitlementRepository;
use crate::repository::export::PgExportRepository;
use crate::repository::health::PgHealthRepository;
use crate::repository::job::PgJobRepository;
use crate::repository::passkey::PgPasskeyRepository;
use crate::repository::session::PgSessionRepository;
use crate::repository::user::{PgUserRepository, UserRepository};
use crate::service::audit::{AuditService, DefaultAuditService};
use crate::service::billing::{BillingProviders, BillingService, DefaultBillingService};
use crate::service::capture::{CaptureProviders, CaptureService, DefaultCaptureService};
use crate::service::design::{DefaultDesignService, DesignProviders, DesignService};
use crate::service::export::{DefaultExportService, ExportProviders, ExportService};
use crate::service::health::{DefaultHealthService, HealthService};
use crate::service::identity::{DefaultIdentityService, IdentityProviders, IdentityService};
use crate::service::job::{DefaultJobService, JobService};
use crate::service::passkey::{DefaultPasskeyService, PasskeyProviders, PasskeyService};
use crate::service::privacy::{DefaultPrivacyService, PrivacyProviders, PrivacyService};
use crate::service::session::{DefaultSessionService, SessionService};
use crate::third_party::email::{EmailSender, SmtpEmailSender};
use crate::third_party::storage::{CloudflareR2, MemoryStorage, ObjectStorage};

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
    /// Photo uploads, vision runs and corrections.
    pub captures: Arc<dyn CaptureService>,
    /// The photo purge (and, later, account deletion).
    pub privacy: Arc<dyn PrivacyService>,
    /// Saved designs.
    pub designs: Arc<dyn DesignService>,
    /// What users have paid for.
    pub billing: Arc<dyn BillingService>,
    /// Exported files.
    pub exports: Arc<dyn ExportService>,
}

/// The outside services the app talks to.
#[derive(Clone)]
pub struct Providers {
    /// Where email goes.
    pub email: Arc<dyn EmailSender>,
    /// Where photos are kept.
    pub storage: Arc<dyn ObjectStorage>,
}

impl Providers {
    /// The providers the settings name: SMTP, and R2 (in development without `STORAGE_*`, a
    /// store in memory, with a warning). Fails only if the HTTPS client can't be set up.
    pub fn from_config(config: &AppConfig) -> Result<Self, AppError> {
        let storage: Arc<dyn ObjectStorage> = match &config.storage {
            Some(storage) => Arc::new(CloudflareR2::new(storage)?),
            None => {
                tracing::warn!(
                    "STORAGE_* is not set: photos are kept in memory and lost at restart"
                );
                Arc::new(MemoryStorage::new())
            }
        };
        Ok(Self {
            email: Arc::new(SmtpEmailSender::new(config.email.clone())),
            storage,
        })
    }
}

impl AppState {
    /// The services wired to `pool` and the providers the settings name.
    pub fn new(config: AppConfig, pool: PgPool) -> Result<Self, AppError> {
        let providers = Providers::from_config(&config)?;
        Ok(Self::with_providers(config, pool, providers))
    }

    /// The services wired to `pool` and `providers` (tests record email and keep photos in
    /// memory).
    pub fn with_providers(config: AppConfig, pool: PgPool, providers: Providers) -> Self {
        let Providers { email, storage } = providers;
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
        let billing: Arc<dyn BillingService> =
            Arc::new(DefaultBillingService::new(BillingProviders {
                db: Arc::clone(&db),
                entitlements: Arc::new(PgEntitlementRepository::new()),
                users: Arc::clone(&users),
                prices: config.prices.clone(),
            }));
        let passkeys = DefaultPasskeyService::new(
            Arc::clone(&config),
            PasskeyProviders {
                db: Arc::clone(&db),
                passkeys: Arc::new(PgPasskeyRepository::new()),
                users,
                sessions: Arc::clone(&sessions),
            },
        );
        let audit = DefaultAuditService::new(Arc::clone(&db), Arc::new(PgAuditRepository::new()));
        let captures: Arc<dyn CaptureRepository> = Arc::new(PgCaptureRepository::new());
        let capture_service = DefaultCaptureService::new((
            CaptureProviders {
                db: Arc::clone(&db),
                captures: Arc::clone(&captures),
                storage: Arc::clone(&storage),
                jobs: Arc::clone(&jobs),
            },
            config.rate_limit_uploads_per_hour,
        ));
        let designs: Arc<dyn DesignService> =
            Arc::new(DefaultDesignService::new(DesignProviders {
                db: Arc::clone(&db),
                designs: Arc::new(PgDesignRepository::new()),
            }));
        let exports = DefaultExportService::new(ExportProviders {
            db: Arc::clone(&db),
            exports: Arc::new(PgExportRepository::new()),
            designs: Arc::clone(&designs),
            billing: Arc::clone(&billing),
            jobs: Arc::clone(&jobs),
            storage: Arc::clone(&storage),
        });
        let privacy = DefaultPrivacyService::new(PrivacyProviders {
            db,
            captures,
            storage,
            jobs: Arc::clone(&jobs),
        });
        Self {
            config,
            health: Arc::new(health),
            jobs,
            sessions,
            identity: Arc::new(identity),
            passkeys: Arc::new(passkeys),
            audit: Arc::new(audit),
            captures: Arc::new(capture_service),
            privacy: Arc::new(privacy),
            designs,
            billing,
            exports: Arc::new(exports),
        }
    }
}
