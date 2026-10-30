//! The client surface, `/api/client/v1`: the web app's users. Domain folders join it as they
//! are built (`export/`, `billing/`, `share/`), each with `<domain>_handler.rs` and
//! `<domain>_route.rs`. Sign-in (`/auth/…`) and the user's account (`/me…`) came with M1-D3,
//! photo uploads (`/captures`) with M1-D4, results and corrections with D5, designs with D6.

pub mod capture;
pub mod design;
pub mod identity;

use axum::{Extension, Router};
use tower_http::set_header::SetResponseHeaderLayer;

use super::{auth, meta, not_found};
use crate::AppState;
use crate::middleware::cors;
use crate::shared::api_surface::{API_VERSION_HEADER, ApiSurface};

/// The client routes, with the web app's CORS origin and the client `api-version` header.
pub fn router(state: &AppState) -> Router<AppState> {
    let surface = ApiSurface::Client;
    Router::new()
        .merge(meta::meta_route::routes(surface))
        .merge(auth::auth_route::routes(
            state,
            (surface, identity::identity_route::sign_in_routes()),
        ))
        .merge(identity::identity_route::account_routes())
        .merge(capture::capture_route::routes(state))
        .merge(design::design_route::routes())
        .fallback(not_found)
        .layer(Extension(surface))
        .layer(SetResponseHeaderLayer::overriding(
            API_VERSION_HEADER,
            surface.version_header(),
        ))
        .layer(cors::for_origin(Some(state.config.web_origin.clone())))
}
