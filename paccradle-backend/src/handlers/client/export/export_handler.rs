use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::{Extension, Json};
use uuid::Uuid;

use crate::AppState;
use crate::error::AppError;
use crate::locales::Locale;
use crate::models::session::AuthUser;
use crate::schemas::common::PageResponse;
use crate::schemas::export::{
    CreateExportRequest, ExportListParams, ExportResponse, ExportSummaryResponse,
};
use crate::service::export::{ExportListRequest, ExportRequest};

/// `POST /designs/{design_id}/exports`: asks for files from the design's current version.
/// 202 with the pending export (poll `GET /exports/{export_id}`); 402 with the checkout when
/// production files aren't covered.
pub async fn create(
    State(state): State<AppState>,
    Extension(locale): Extension<Locale>,
    user: AuthUser,
    Path(design_id): Path<Uuid>,
    Json(body): Json<CreateExportRequest>,
) -> Result<(StatusCode, Json<ExportResponse>), AppError> {
    let ask = body.ask()?;
    let request = ExportRequest {
        user: &user,
        design_id,
        formats: ask.formats,
        paper: ask.paper,
        version: ask.version,
    };
    let view = state.exports.request(request).await?;
    Ok((
        StatusCode::ACCEPTED,
        Json(ExportResponse::new((view, locale))),
    ))
}

/// `GET /exports/{export_id}`: the caller's export, with 5-minute links once it is done.
pub async fn get(
    State(state): State<AppState>,
    Extension(locale): Extension<Locale>,
    user: AuthUser,
    Path(export_id): Path<Uuid>,
) -> Result<Json<ExportResponse>, AppError> {
    let view = state.exports.get((&user, export_id)).await?;
    Ok(Json(ExportResponse::new((view, locale))))
}

/// `GET /designs/{design_id}/exports`: a page of the design's exports, newest first.
pub async fn list(
    State(state): State<AppState>,
    user: AuthUser,
    Path(design_id): Path<Uuid>,
    Query(params): Query<ExportListParams>,
) -> Result<Json<PageResponse<ExportSummaryResponse>>, AppError> {
    let (page, status, order) = params.filters()?;
    let request = ExportListRequest {
        user: &user,
        design_id,
        page,
        status,
        order,
    };
    let page = state.exports.list(request).await?;
    Ok(Json(PageResponse::new((page, ExportSummaryResponse::from))))
}
