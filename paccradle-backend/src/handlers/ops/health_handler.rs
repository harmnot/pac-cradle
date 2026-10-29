use axum::Json;
use axum::extract::State;

use crate::AppState;
use crate::error::AppError;
use crate::schemas::meta::{HealthResponse, HealthStatus};

/// `GET /healthz`: the process is up and answering. It checks nothing else, so a database
/// outage doesn't get the container restarted.
pub async fn health() -> Json<HealthResponse> {
    Json(ok())
}

/// `GET /readyz`: the API can serve requests, the database included. The deploy waits for it
/// before switching over; 503 `SERVICE_UNAVAILABLE` names what isn't ready.
pub async fn ready(State(state): State<AppState>) -> Result<Json<HealthResponse>, AppError> {
    state.health.readiness().await?;
    Ok(Json(ok()))
}

fn ok() -> HealthResponse {
    HealthResponse {
        status: HealthStatus::Ok,
        version: env!("CARGO_PKG_VERSION"),
    }
}
