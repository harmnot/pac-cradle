use serde::Serialize;

use crate::shared::api_surface::ApiSurface;

/// `GET /healthz` and `GET /readyz`.
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    /// Always `ok`: a failing check answers 503 with an error body instead.
    pub status: HealthStatus,
    /// The release version (`Cargo.toml`), so a deploy can see which build answers.
    pub version: &'static str,
}

/// A passing health check.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    /// The check passed.
    Ok,
}

/// `GET /api/{surface}/v{major}/version`.
#[derive(Debug, Serialize)]
pub struct VersionResponse {
    /// `client` or `admin`.
    pub surface: ApiSurface,
    /// The surface's semver contract version, the same as the `api-version` header.
    pub version: &'static str,
}

impl From<ApiSurface> for VersionResponse {
    fn from(surface: ApiSurface) -> Self {
        Self {
            surface,
            version: surface.version(),
        }
    }
}
