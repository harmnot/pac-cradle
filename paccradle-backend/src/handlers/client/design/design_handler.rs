use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use uuid::Uuid;

use crate::AppState;
use crate::error::AppError;
use crate::models::design::{DesignSummary, SavedDesign};
use crate::models::session::AuthUser;
use crate::schemas::common::PageResponse;
use crate::schemas::design::{
    CreateDesignRequest, DesignListParams, DesignResponse, DesignSummaryResponse,
    UpdateDesignRequest,
};
use crate::service::design::{DesignListRequest, DesignUpdate, NewDesignRequest};

/// `POST /designs`: saves the editor's design at version 1. 201. Guests may save.
pub async fn create(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<CreateDesignRequest>,
) -> Result<(StatusCode, Json<DesignResponse>), AppError> {
    let (name, spec) = body.parts()?;
    let request = NewDesignRequest {
        user: &user,
        name,
        spec,
    };
    let design = state.designs.create(request).await?;
    Ok((
        StatusCode::CREATED,
        Json(DesignResponse::new((design, None))),
    ))
}

/// `GET /designs`: a page of the caller's designs ("My designs"), filtered and sorted.
pub async fn list(
    State(state): State<AppState>,
    user: AuthUser,
    Query(params): Query<DesignListParams>,
) -> Result<Json<PageResponse<DesignSummaryResponse>>, AppError> {
    let filters = params.filters()?;
    let request = DesignListRequest {
        user: &user,
        page: filters.page,
        search: filters.search,
        style: filters.style,
        sort: filters.sort,
        order: filters.order,
    };
    let page = state.designs.list(request).await?;
    let ids: Vec<Uuid> = page.items.iter().map(|design| design.id).collect();
    let unlocks = state.billing.unlocks((&user, &ids)).await?;
    Ok(Json(PageResponse::new((page, |design: DesignSummary| {
        let unlock = unlocks.get(&design.id);
        DesignSummaryResponse::new((design, unlock))
    }))))
}

/// `GET /designs/{design_id}`: the caller's design, with its unlock (poll it after a checkout).
pub async fn get(
    State(state): State<AppState>,
    user: AuthUser,
    Path(design_id): Path<Uuid>,
) -> Result<Json<DesignResponse>, AppError> {
    let design = state.designs.get((&user, design_id)).await?;
    Ok(Json(with_unlock((&state, &user, design)).await?))
}

/// `PATCH /designs/{design_id}`: saves a change made from `version`; the answer carries the
/// next version to send.
pub async fn update(
    State(state): State<AppState>,
    user: AuthUser,
    Path(design_id): Path<Uuid>,
    Json(body): Json<UpdateDesignRequest>,
) -> Result<Json<DesignResponse>, AppError> {
    let request = DesignUpdate {
        user: &user,
        design_id,
        change: body.change()?,
    };
    let design = state.designs.update(request).await?;
    Ok(Json(with_unlock((&state, &user, design)).await?))
}

/// The design's response, with its unlock if it has one.
async fn with_unlock(
    (state, user, design): (&AppState, &AuthUser, SavedDesign),
) -> Result<DesignResponse, AppError> {
    let unlocks = state.billing.unlocks((user, &[design.id])).await?;
    let unlock = unlocks.get(&design.id);
    Ok(DesignResponse::new((design, unlock)))
}
