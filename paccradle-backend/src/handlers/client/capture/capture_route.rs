use std::num::NonZeroU32;
use std::time::Duration;

use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::http::HeaderValue;
use axum::http::header::CACHE_CONTROL;
use axum::routing::{get, post};
use tower_http::set_header::SetResponseHeaderLayer;

use super::capture_handler;
use crate::AppState;
use crate::config::UPLOADS_PER_ADDRESS_FACTOR;
use crate::middleware::rate_limit::{self, PerClient};
use crate::service::capture::MAX_PHOTO_BYTES;

/// A photo, the `capture` settings (16 KiB at most) and the multipart wrapping round them.
pub const UPLOAD_BODY_LIMIT: usize = MAX_PHOTO_BYTES + 64 * 1024;

const HOUR: Duration = Duration::from_secs(60 * 60);

/// `POST /captures`, with its own body limit (the API-wide one is 1 MiB) and an hourly
/// per-address upload limit, checked before the body is read (the per-user limit is the
/// capture service's); `GET /captures/{capture_id}` and `POST /captures/{capture_id}/correct`,
/// answered `no-store` (they carry a signed photo link).
///
/// The API-wide 30-second timeout still applies: Nginx takes the whole upload from the
/// phone before passing it on (request buffering), so the API reads it from localhost in
/// milliseconds.
pub fn routes(state: &AppState) -> Router<AppState> {
    let per_address = state
        .config
        .rate_limit_uploads_per_hour
        .saturating_mul(NonZeroU32::new(UPLOADS_PER_ADDRESS_FACTOR).unwrap_or(NonZeroU32::MIN));
    let uploads = PerClient::with_window(&state.config, (per_address, HOUR));
    let upload = Router::new().route(
        "/captures",
        post(capture_handler::create)
            .layer(DefaultBodyLimit::max(UPLOAD_BODY_LIMIT))
            .layer(axum::middleware::from_fn_with_state(
                uploads,
                rate_limit::per_client,
            )),
    );
    let capture = Router::new()
        .route("/captures/{capture_id}", get(capture_handler::get))
        .route(
            "/captures/{capture_id}/correct",
            post(capture_handler::correct),
        )
        .layer(SetResponseHeaderLayer::overriding(
            CACHE_CONTROL,
            HeaderValue::from_static("no-store"),
        ));
    upload.merge(capture)
}
