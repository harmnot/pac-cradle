//! CORS per surface: the client API answers only the web app's origin (`APP_WEB_URL`), the
//! admin API only the back office's (`ADMIN_WEB_URL`, none when unset). Credentials are
//! allowed for the refresh cookie (M1-D3), so no wildcard is ever used.

use std::time::Duration;

use axum::http::header::{ACCEPT, ACCEPT_LANGUAGE, AUTHORIZATION, CONTENT_TYPE, RETRY_AFTER};
use axum::http::{HeaderValue, Method};
use tower_http::cors::{AllowOrigin, CorsLayer};

use super::request_id::REQUEST_ID_HEADER;
use crate::shared::api_surface::API_VERSION_HEADER;

/// How long browsers may reuse a preflight answer.
const PREFLIGHT_MAX_AGE: Duration = Duration::from_secs(600);

/// CORS for one browser origin, or for none.
pub fn for_origin(origin: Option<HeaderValue>) -> CorsLayer {
    CorsLayer::new()
        .allow_origin(AllowOrigin::list(origin))
        .allow_credentials(true)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
        ])
        .allow_headers([ACCEPT, ACCEPT_LANGUAGE, AUTHORIZATION, CONTENT_TYPE])
        .expose_headers([REQUEST_ID_HEADER, API_VERSION_HEADER, RETRY_AFTER])
        .max_age(PREFLIGHT_MAX_AGE)
}
