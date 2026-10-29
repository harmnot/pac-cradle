use axum::Json;

use crate::schemas::meta::VersionResponse;
use crate::shared::api_surface::ApiSurface;

/// `GET /api/{surface}/v{major}/version`: the surface and its full contract version.
pub fn version(surface: ApiSurface) -> Json<VersionResponse> {
    Json(VersionResponse::from(surface))
}
