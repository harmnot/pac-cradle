use axum::Router;
use axum::routing::get;

use super::design_handler;
use crate::AppState;

/// `POST`/`GET /designs` and `GET`/`PATCH /designs/{design_id}`.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/designs",
            get(design_handler::list).post(design_handler::create),
        )
        .route(
            "/designs/{design_id}",
            get(design_handler::get).patch(design_handler::update),
        )
}
