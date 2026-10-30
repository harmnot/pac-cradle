//! The API's surfaces and their contract versions (`backend-architecture.md` §4.8).
//!
//! Each surface is versioned on its own with semver. The URL carries only the major version
//! (`/api/client/v1/…`); every response from a surface carries the full version in the
//! `api-version` header.
//!
//! - **Patch** (`1.0.1`): a fix that changes no request or response shape.
//! - **Minor** (`1.1.0`): something added that old callers can ignore: an endpoint, an optional
//!   request field, a response field, an error code.
//! - **Major** (`2.0.0`): anything that can break a caller. It gets a new prefix (`/v2`), and
//!   the old one keeps running until its callers have moved.
//! - **Pre-release** (`1.0.0-beta.N`): before the public launch (M2) nothing outside our own
//!   web app calls the API, so the contract may still change in any way; each change bumps `N`.

use axum::http::{HeaderName, HeaderValue};
use serde::Serialize;

/// The response header carrying a surface's full contract version.
pub const API_VERSION_HEADER: HeaderName = HeaderName::from_static("api-version");

/// Who calls a group of routes. Each surface has its own routes, URL prefix, CORS origin,
/// authentication and contract version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiSurface {
    /// The web app's users: capture, design, export, billing, `/me`.
    Client,
    /// The back office: support, refunds and credits, jobs, accuracy reviews. Signed-in admins only.
    Admin,
}

impl ApiSurface {
    /// Every surface.
    pub const ALL: [Self; 2] = [Self::Client, Self::Admin];

    /// The URL segment after `/api/`.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Client => "client",
            Self::Admin => "admin",
        }
    }

    /// The surface's semver contract version. Bump it with every change to its routes,
    /// requests, responses or error codes, and write the change into a dated `postman/` file.
    pub const fn version(self) -> &'static str {
        match self {
            Self::Client => "1.0.0-beta.6",
            Self::Admin => "1.0.0-beta.2",
        }
    }

    /// The major version, the number in the URL.
    pub fn major(self) -> &'static str {
        let version = self.version();
        version.split_once('.').map_or(version, |(major, _)| major)
    }

    /// Where the surface's routes live, e.g. `/api/client/v1`.
    pub fn prefix(self) -> String {
        format!("/api/{}/v{}", self.name(), self.major())
    }

    /// The `api-version` header value.
    pub const fn version_header(self) -> HeaderValue {
        HeaderValue::from_static(self.version())
    }
}
