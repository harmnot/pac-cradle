use axum::Router;
use axum::http::HeaderValue;
use axum::http::header::CACHE_CONTROL;
use axum::routing::get;
use tower_http::set_header::SetResponseHeaderLayer;

use super::export_handler;
use crate::AppState;

/// `POST`/`GET /designs/{design_id}/exports` and `GET /exports/{export_id}`, answered
/// `no-store` (they carry signed download links).
pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/designs/{design_id}/exports",
            get(export_handler::list).post(export_handler::create),
        )
        .route("/exports/{export_id}", get(export_handler::get))
        .layer(SetResponseHeaderLayer::overriding(
            CACHE_CONTROL,
            HeaderValue::from_static("no-store"),
        ))
}
