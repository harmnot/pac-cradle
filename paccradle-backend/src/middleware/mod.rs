//! Cross-cutting HTTP behaviour (`backend-architecture.md` §4.10). [`stack`] wraps every
//! route; [`cors`], [`admin_guard`], [`same_origin`] and the sign-in rate limit are added per
//! surface in `handlers`; [`auth`] reads who a request comes from.

pub mod admin_guard;
pub mod auth;
pub mod client_ip;
pub mod compression;
pub mod cors;
pub mod error_body;
pub mod rate_limit;
pub mod request_id;
pub mod same_origin;
pub mod timeout;
pub mod trace;

use axum::Router;
use axum::extract::DefaultBodyLimit;
use tower::ServiceBuilder;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::request_id::{PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::trace::TraceLayer;

use crate::config::AppConfig;

/// Wraps `router` in the middleware every request goes through, outermost first:
///
/// 1. drop any `x-request-id` the caller sent, then give the request a UUID v7 one;
/// 2. trace it (method, path without the query, request ID, status, latency);
/// 3. send the request ID back in `x-request-id`;
/// 4. compress the response (gzip or brotli), except one that sets a cookie;
/// 5. write error bodies as JSON in the caller's language;
/// 6. turn a panic into a 500;
/// 7. give up after `HTTP_REQUEST_TIMEOUT_SECONDS` with a 503;
/// 8. limit requests per client address;
/// 9. cap request bodies at `HTTP_BODY_LIMIT_BYTES`.
pub fn stack(router: Router, config: &AppConfig) -> Router {
    router.layer(
        ServiceBuilder::new()
            .layer(axum::middleware::map_request(
                request_id::drop_client_request_id,
            ))
            .layer(SetRequestIdLayer::x_request_id(
                request_id::MakeRequestUuidV7,
            ))
            .layer(
                TraceLayer::new_for_http()
                    .make_span_with(trace::make_span)
                    .on_response(trace::on_response())
                    .on_failure(()),
            )
            .layer(PropagateRequestIdLayer::x_request_id())
            .layer(compression::layer())
            .layer(axum::middleware::from_fn(error_body::render))
            .layer(CatchPanicLayer::new())
            .layer(axum::middleware::from_fn_with_state(
                config.request_timeout,
                timeout::limit,
            ))
            .layer(axum::middleware::from_fn_with_state(
                rate_limit::PerClient::new(config),
                rate_limit::per_client,
            ))
            .layer(DefaultBodyLimit::max(config.http_body_limit_bytes)),
    )
}
