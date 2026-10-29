//! The caller's address, for rate limits. It is never logged.

use std::net::{IpAddr, SocketAddr};

use axum::extract::{ConnectInfo, Request};
use axum::http::HeaderName;

/// The address in `trusted_header` (set by Nginx, which overwrites what callers send), or the
/// connection's peer address when that header is unset, missing or unreadable. `None` only
/// when the server runs without connection info (tests).
pub fn client_ip(request: &Request, trusted_header: Option<&HeaderName>) -> Option<IpAddr> {
    let from_header = trusted_header
        .and_then(|name| request.headers().get(name))
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .and_then(|value| value.trim().parse().ok());
    from_header.or_else(|| {
        request
            .extensions()
            .get::<ConnectInfo<SocketAddr>>()
            .map(|ConnectInfo(peer)| peer.ip())
    })
}
