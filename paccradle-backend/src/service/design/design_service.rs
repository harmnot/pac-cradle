use std::sync::Arc;

use insert_core::DesignError;
use uuid::Uuid;

use super::{DesignListRequest, DesignService, DesignUpdate, NewDesignRequest};
use crate::error::{AppError, ErrorCode};
use crate::models::design::{DesignListQuery, DesignSummary, NewDesign, SavedDesign};
use crate::models::page::Page;
use crate::models::session::AuthUser;
use crate::repository::database::Database;
use crate::repository::design::DesignRepository;
use crate::shared::future::BoxFuture;
use crate::shared::id::new_id;

/// What [`DefaultDesignService`] works with.
pub struct DesignProviders {
    /// Connections and transactions.
    pub db: Arc<dyn Database>,
    /// `designs`.
    pub designs: Arc<dyn DesignRepository>,
}

/// [`DesignService`] on Postgres, checking specs with `insert_core`.
pub struct DefaultDesignService {
    parts: DesignProviders,
}

impl DefaultDesignService {
    /// A service on `parts`.
    pub fn new(parts: DesignProviders) -> Self {
        Self { parts }
    }
}

impl DesignService for DefaultDesignService {
    fn create<'a>(
        &'a self,
        request: NewDesignRequest<'a>,
    ) -> BoxFuture<'a, Result<SavedDesign, AppError>> {
        Box::pin(async move {
            request.spec.validate().map_err(invalid)?;
            let design = NewDesign {
                id: new_id(),
                user_id: request.user.user_id,
                name: request.name,
                spec: request.spec,
            };
            let mut conn = self.parts.db.connection().await?;
            self.parts.designs.insert(&mut conn, &design).await
        })
    }

    fn get<'a>(
        &'a self,
        (user, id): (&'a AuthUser, Uuid),
    ) -> BoxFuture<'a, Result<SavedDesign, AppError>> {
        Box::pin(async move {
            let mut conn = self.parts.db.connection().await?;
            owned((self.parts.designs.find(&mut conn, id).await?, user))
        })
    }

    fn update<'a>(
        &'a self,
        request: DesignUpdate<'a>,
    ) -> BoxFuture<'a, Result<SavedDesign, AppError>> {
        Box::pin(async move {
            let DesignUpdate {
                user,
                design_id,
                change,
            } = request;
            if let Some(spec) = &change.spec {
                spec.validate().map_err(invalid)?;
            }
            let mut tx = self.parts.db.begin().await?;
            let saved = owned((self.parts.designs.lock(&mut tx, design_id).await?, user))?;
            if saved.version != change.from_version {
                return Err(AppError::new(ErrorCode::DesignVersionConflict)
                    .with_meta("current_version", saved.version));
            }
            let mut next = saved.clone();
            if let Some(name) = change.name {
                next.name = name;
            }
            if let Some(spec) = change.spec {
                next.spec = spec;
            }
            if next == saved {
                return Ok(saved);
            }
            next.version += 1;
            let stored = self.parts.designs.update(&mut tx, &next).await?;
            tx.commit().await?;
            Ok(stored)
        })
    }

    fn list<'a>(
        &'a self,
        request: DesignListRequest<'a>,
    ) -> BoxFuture<'a, Result<Page<DesignSummary>, AppError>> {
        Box::pin(async move {
            let query = DesignListQuery {
                user_id: request.user.user_id,
                page: request.page,
                search: request.search,
                style: request.style,
                sort: request.sort,
                order: request.order,
            };
            let mut conn = self.parts.db.connection().await?;
            let total = self.parts.designs.count(&mut conn, &query).await?;
            let items = self.parts.designs.list(&mut conn, &query).await?;
            Ok(Page {
                items,
                request: query.page,
                total,
            })
        })
    }
}

/// The design when it belongs to `user`; `DESIGN_NOT_FOUND` otherwise, so nobody learns
/// whether someone else's exists.
fn owned((design, user): (Option<SavedDesign>, &AuthUser)) -> Result<SavedDesign, AppError> {
    match design {
        Some(design) if design.user_id == user.user_id => Ok(design),
        _ => Err(AppError::new(ErrorCode::DesignNotFound)),
    }
}

/// `DESIGN_INVALID`, with `insert_core`'s own words for what is wrong.
fn invalid(error: DesignError) -> AppError {
    AppError::new(ErrorCode::DesignInvalid).with_meta("detail", error.to_string())
}
