//! Requests per client address across the whole API (`RATE_LIMIT_PER_MINUTE`), and the
//! stricter one on sign-in routes (`RATE_LIMIT_AUTH_PER_MINUTE`, M1-D3). Uploads (D4) can
//! reuse [`PerClient::with_rate`].

use std::net::{IpAddr, Ipv4Addr};
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::extract::{Request, State};
use axum::http::header::RETRY_AFTER;
use axum::http::{HeaderName, HeaderValue};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use super::client_ip::client_ip;
use crate::config::AppConfig;
use crate::error::{AppError, ErrorCode};
use crate::shared::rate_limiter::RateLimiter;

/// The limiter and where to read the client address from.
#[derive(Clone)]
pub struct PerClient {
    limiter: Arc<RateLimiter<IpAddr>>,
    client_ip_header: Option<HeaderName>,
}

impl PerClient {
    /// A limiter with the configured rate and client address header.
    pub fn new(config: &AppConfig) -> Self {
        Self::with_rate(config, config.rate_limit_per_minute)
    }

    /// A limiter allowing `per_minute` per client address (the sign-in routes' own limit),
    /// reading the address like the API-wide one.
    pub fn with_rate(config: &AppConfig, per_minute: NonZeroU32) -> Self {
        Self {
            limiter: Arc::new(RateLimiter::new(per_minute)),
            client_ip_header: config.client_ip_header.clone(),
        }
    }
}

/// Answers `RATE_LIMITED` (429, with `retry-after`) once a client address has used its requests.
pub async fn per_client(State(state): State<PerClient>, request: Request, next: Next) -> Response {
    // Requests without a known address (none in production) share one bucket.
    let ip = client_ip(&request, state.client_ip_header.as_ref())
        .unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED));
    match state.limiter.check(ip, Instant::now()) {
        Ok(()) => next.run(request).await,
        Err(wait) => too_many_requests(wait),
    }
}

/// A `RATE_LIMITED` response telling the caller to wait `wait`, rounded up to whole seconds.
pub fn too_many_requests(wait: Duration) -> Response {
    let seconds = (wait.as_secs() + u64::from(wait.subsec_nanos() > 0)).max(1);
    let mut response = AppError::new(ErrorCode::RateLimited)
        .with_meta("retry_after_seconds", seconds)
        .into_response();
    response
        .headers_mut()
        .insert(RETRY_AFTER, HeaderValue::from(seconds));
    response
}
