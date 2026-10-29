//! The connection pool and the migrations in `migrations/`, which are built into the binary.

use std::str::FromStr;
use std::time::Duration;

use sqlx::migrate::MigrateError;
use sqlx::pool::PoolConnection;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{PgPool, Postgres, Transaction};

use crate::config::DatabaseConfig;
use crate::error::AppError;
use crate::shared::future::BoxFuture;

/// An open transaction. Pass `&mut tx` (it derefs to a connection) to repository methods, then
/// `tx.commit()`; dropping it rolls back.
pub type Tx = Transaction<'static, Postgres>;

/// A pooled connection, for services that run single statements outside a transaction.
pub type Conn = PoolConnection<Postgres>;

/// Connections and transactions for services. Repository methods take `&mut PgConnection`, so
/// the service decides whether a group of statements commits together.
pub trait Database: Send + Sync {
    /// Starts a transaction.
    fn begin(&self) -> BoxFuture<'_, Result<Tx, AppError>>;

    /// A connection from the pool.
    fn connection(&self) -> BoxFuture<'_, Result<Conn, AppError>>;
}

/// [`Database`] on a Postgres pool.
pub struct PgDatabase {
    pool: PgPool,
}

impl PgDatabase {
    /// Transactions and connections from `pool`.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl Database for PgDatabase {
    fn begin(&self) -> BoxFuture<'_, Result<Tx, AppError>> {
        Box::pin(async move { Ok(self.pool.begin().await?) })
    }

    fn connection(&self) -> BoxFuture<'_, Result<Conn, AppError>> {
        Box::pin(async move { Ok(self.pool.acquire().await?) })
    }
}

/// How long a request waits for a free connection before it fails with 503.
const ACQUIRE_TIMEOUT: Duration = Duration::from_secs(5);

/// The longest a single statement may run, matching the HTTP request timeout.
const STATEMENT_TIMEOUT: &str = "30s";

/// Opens the pool and checks that a connection works.
pub async fn connect(config: &DatabaseConfig) -> Result<PgPool, sqlx::Error> {
    pool_options(config)
        .connect_with(connect_options(config)?)
        .await
}

/// A pool that connects on first use, for servers and tests that may never need it.
pub fn connect_lazy(config: &DatabaseConfig) -> Result<PgPool, sqlx::Error> {
    Ok(pool_options(config).connect_lazy_with(connect_options(config)?))
}

/// Applies the migrations not yet applied. Postgres holds a lock meanwhile, so two servers
/// starting together don't both run them.
pub async fn migrate(pool: &PgPool) -> Result<(), MigrateError> {
    sqlx::migrate!().run(pool).await
}

fn pool_options(config: &DatabaseConfig) -> PgPoolOptions {
    PgPoolOptions::new()
        .max_connections(config.max_connections)
        .acquire_timeout(ACQUIRE_TIMEOUT)
}

fn connect_options(config: &DatabaseConfig) -> Result<PgConnectOptions, sqlx::Error> {
    Ok(PgConnectOptions::from_str(config.url.expose())?
        .application_name("paccradle-api")
        .options([("statement_timeout", STATEMENT_TIMEOUT)]))
}
