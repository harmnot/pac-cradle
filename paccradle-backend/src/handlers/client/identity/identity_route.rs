use axum::Router;
use axum::http::HeaderValue;
use axum::http::header::CACHE_CONTROL;
use axum::routing::{get, post, put};
use tower_http::set_header::SetResponseHeaderLayer;

use super::identity_handler;
use crate::AppState;

/// The web app's own sign-ins, mounted with the shared `/auth` routes (and their limits).
pub fn sign_in_routes() -> Router<AppState> {
    Router::new()
        .route("/auth/guest", post(identity_handler::start_guest))
        .route(
            "/auth/email-link",
            post(identity_handler::request_email_link),
        )
        .route(
            "/auth/email-link/sign-in",
            post(identity_handler::email_link_sign_in),
        )
        .route(
            "/auth/password/sign-in",
            post(identity_handler::password_sign_in),
        )
}

/// The signed-in user's account, answered `no-store`.
pub fn account_routes() -> Router<AppState> {
    Router::new()
        .route("/me", get(identity_handler::me))
        .route("/me/password", put(identity_handler::set_password))
        .route(
            "/me/passkeys/options",
            post(identity_handler::passkey_options),
        )
        .route("/me/passkeys", post(identity_handler::add_passkey))
        .layer(SetResponseHeaderLayer::overriding(
            CACHE_CONTROL,
            HeaderValue::from_static("no-store"),
        ))
}
