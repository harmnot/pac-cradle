use axum::Router;
use axum::routing::get;

use super::health_handler;
use crate::AppState;

/// `GET /healthz` and `GET /readyz`.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/healthz", get(health_handler::health))
        .route("/readyz", get(health_handler::ready))
}
