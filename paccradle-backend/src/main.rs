use std::net::SocketAddr;
use std::process::ExitCode;
use std::time::Duration;

use paccradle_backend::AppState;
use paccradle_backend::config::AppConfig;
use paccradle_backend::jobs::{self, Worker};
use paccradle_backend::repository::database;
use paccradle_backend::shared::{logger, shutdown};
use tokio::net::TcpListener;
use tokio::sync::watch;

/// How long running jobs get to finish after the server stops. The deploy allows 35 s in all.
const WORKER_GRACE: Duration = Duration::from_secs(25);

#[tokio::main]
async fn main() -> ExitCode {
    let config = match AppConfig::load() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("paccradle API: bad setting, {error}");
            return ExitCode::FAILURE;
        }
    };
    logger::init(config.env);

    let pool = match database::connect(&config.database).await {
        Ok(pool) => pool,
        Err(error) => {
            tracing::error!(?error, "cannot reach the database (DATABASE_URL)");
            return ExitCode::FAILURE;
        }
    };
    if config.database.run_migrations
        && let Err(error) = database::migrate(&pool).await
    {
        tracing::error!(?error, "cannot apply the migrations");
        return ExitCode::FAILURE;
    }
    tracing::info!(
        migrations = config.database.run_migrations,
        "database ready"
    );

    let address = SocketAddr::new(config.host, config.port);
    let listener = match TcpListener::bind(address).await {
        Ok(listener) => listener,
        Err(error) => {
            tracing::error!(%address, %error, "cannot listen");
            return ExitCode::FAILURE;
        }
    };

    let job_workers = config.job_workers;
    let state = AppState::new(config, pool);
    let (stop_sender, stop) = watch::channel(false);
    tokio::spawn(async move {
        shutdown::signal().await;
        tracing::info!("shutting down");
        let _ = stop_sender.send(true);
    });
    let workers =
        Worker::new(state.jobs.clone(), jobs::handlers(&state)).start(job_workers, stop.clone());
    tracing::info!(%address, version = env!("CARGO_PKG_VERSION"), job_workers, "paccradle API listening");

    // Connection info gives the rate limiter the peer address when no proxy header is set.
    let app = paccradle_backend::app(state).into_make_service_with_connect_info::<SocketAddr>();
    let served = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown::stopped(stop))
        .await;

    let finished = tokio::time::timeout(WORKER_GRACE, async {
        for worker in workers {
            let _ = worker.await;
        }
    })
    .await;
    if finished.is_err() {
        tracing::warn!("jobs still running at exit; they run again when their lease ends");
    }
    if let Err(error) = served {
        tracing::error!(%error, "server stopped");
        return ExitCode::FAILURE;
    }
    tracing::info!("paccradle API stopped");
    ExitCode::SUCCESS
}
