//! HTTP routes. Each surface has its own router, nested under its prefix:
//!
//! - `/healthz`, `/readyz`: probes for the deploy scripts, outside both surfaces;
//! - `/api/client/v1/…`: [`client`], for the web app's users;
//! - `/api/admin/v1/…`: [`admin`], for the back office.

pub mod admin;
pub mod auth;
pub mod client;
pub mod meta;
pub mod ops;

use axum::Router;
use axum::response::IntoResponse;

use crate::AppState;
use crate::error::{AppError, ErrorCode};
use crate::shared::api_surface::ApiSurface;

/// Every route of the API.
pub fn router(state: AppState) -> Router {
    Router::new()
        .merge(ops::health_route::routes())
        .nest(&ApiSurface::Client.prefix(), client::router(&state))
        .nest(&ApiSurface::Admin.prefix(), admin::router(&state))
        .fallback(not_found)
        .with_state(state)
}

/// `ROUTE_NOT_FOUND` for any path no route matches.
pub async fn not_found() -> impl IntoResponse {
    AppError::new(ErrorCode::RouteNotFound)
}
