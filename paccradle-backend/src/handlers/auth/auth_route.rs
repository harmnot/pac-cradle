use axum::Router;
use axum::http::HeaderValue;
use axum::http::header::CACHE_CONTROL;
use axum::routing::post;
use tower_http::set_header::SetResponseHeaderLayer;

use super::auth_handler;
use crate::AppState;
use crate::middleware::rate_limit::{self, PerClient};
use crate::middleware::same_origin;
use crate::shared::api_surface::ApiSurface;

/// The `/auth` routes of `surface`: passkey sign-in, refresh and sign-out, plus `more_sign_in`
/// (the web app's guest, email-link and password sign-ins). Sign-ins share one per-address
/// limit (`RATE_LIMIT_AUTH_PER_MINUTE`); every `/auth` route refuses other sites' `Origin`
/// and answers `no-store`.
pub fn routes(
    state: &AppState,
    (surface, more_sign_in): (ApiSurface, Router<AppState>),
) -> Router<AppState> {
    let config = &state.config;
    let web_origin = match surface {
        ApiSurface::Client => Some(config.web_origin.clone()),
        ApiSurface::Admin => config.admin_web_origin.clone(),
    };
    let sign_in = Router::new()
        .route(
            "/auth/passkey/sign-in/options",
            post(auth_handler::passkey_options),
        )
        .route("/auth/passkey/sign-in", post(auth_handler::passkey_sign_in))
        .merge(more_sign_in)
        .layer(axum::middleware::from_fn_with_state(
            PerClient::with_rate(config, config.rate_limit_auth_per_minute),
            rate_limit::per_client,
        ));
    let session = Router::new()
        .route("/auth/refresh", post(auth_handler::refresh))
        .route("/auth/sign-out", post(auth_handler::sign_out));
    sign_in
        .merge(session)
        .layer(axum::middleware::from_fn_with_state(
            web_origin,
            same_origin::require,
        ))
        .layer(SetResponseHeaderLayer::overriding(
            CACHE_CONTROL,
            HeaderValue::from_static("no-store"),
        ))
}
