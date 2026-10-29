use axum::Json;
use axum::extract::State;

use crate::AppState;
use crate::error::AppError;
use crate::models::session::AuthUser;
use crate::schemas::identity::AccountResponse;

/// `GET /me`: the signed-in admin.
pub async fn me(
    State(state): State<AppState>,
    admin: AuthUser,
) -> Result<Json<AccountResponse>, AppError> {
    let account = state.identity.account(&admin).await?;
    Ok(Json(AccountResponse::from(account)))
}
