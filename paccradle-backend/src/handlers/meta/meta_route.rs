use axum::Router;
use axum::routing::get;

use super::meta_handler;
use crate::AppState;
use crate::shared::api_surface::ApiSurface;

/// `GET /version` for `surface`.
pub fn routes(surface: ApiSurface) -> Router<AppState> {
    Router::new().route(
        "/version",
        get(move || async move { meta_handler::version(surface) }),
    )
}
