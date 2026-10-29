//! The admin surface, `/api/admin/v1`: the back office. Every path, known or not, sits behind
//! [`require_admin`] except passkey sign-in, refresh and sign-out (`/auth/…`), and browsers may
//! call it only from `ADMIN_WEB_URL`. Admin domain folders join it as they are built, each with
//! `<domain>_handler.rs` and `<domain>_route.rs`; they call the same services as the client
//! handlers.

pub mod identity;

use axum::{Extension, Router};
use tower_http::set_header::SetResponseHeaderLayer;

use super::{auth, meta, not_found};
use crate::AppState;
use crate::middleware::admin_guard::require_admin;
use crate::middleware::cors;
use crate::shared::api_surface::{API_VERSION_HEADER, ApiSurface};

/// The admin routes: sign-in open, everything else behind the admin guard (which also logs
/// writes), then the admin `api-version` header, then CORS outermost so browser preflights
/// are answered without credentials.
pub fn router(state: &AppState) -> Router<AppState> {
    let surface = ApiSurface::Admin;
    let guarded = Router::new()
        .merge(meta::meta_route::routes(surface))
        .merge(identity::identity_route::routes())
        .fallback(not_found)
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            require_admin,
        ));
    Router::new()
        .merge(auth::auth_route::routes(state, (surface, Router::new())))
        .merge(guarded)
        .layer(Extension(surface))
        .layer(SetResponseHeaderLayer::overriding(
            API_VERSION_HEADER,
            surface.version_header(),
        ))
        .layer(cors::for_origin(state.config.admin_web_origin.clone()))
}
