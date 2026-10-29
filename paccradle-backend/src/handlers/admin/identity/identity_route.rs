use axum::Router;
use axum::http::HeaderValue;
use axum::http::header::CACHE_CONTROL;
use axum::routing::get;
use tower_http::set_header::SetResponseHeaderLayer;

use super::identity_handler;
use crate::AppState;

/// `GET /me`, behind the admin guard, answered `no-store`.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/me", get(identity_handler::me))
        .layer(SetResponseHeaderLayer::overriding(
            CACHE_CONTROL,
            HeaderValue::from_static("no-store"),
        ))
}
