use uuid::Uuid;

use crate::error::AppError;
use crate::models::design::{DesignChange, DesignSort, DesignSummary, InsertStyle, SavedDesign};
use crate::models::page::{Page, PageRequest, SortOrder};
use crate::models::session::AuthUser;
use crate::shared::future::BoxFuture;

/// A design to save for the first time.
#[derive(Debug, Clone)]
pub struct NewDesignRequest<'a> {
    /// Whose it is (a guest too).
    pub user: &'a AuthUser,
    /// Its name, already trimmed and checked; may be empty.
    pub name: String,
    /// What the editor made; checked here.
    pub spec: insert_core::Design,
}

/// A change to one of the caller's designs.
#[derive(Debug, Clone)]
pub struct DesignUpdate<'a> {
    /// Who asks.
    pub user: &'a AuthUser,
    /// Which design.
    pub design_id: Uuid,
    /// What changes, from which version. A new spec is checked here.
    pub change: DesignChange,
}

/// A page of the caller's designs.
#[derive(Debug, Clone)]
pub struct DesignListRequest<'a> {
    /// Whose designs.
    pub user: &'a AuthUser,
    /// Which page.
    pub page: PageRequest,
    /// Only names containing this, ignoring letter case.
    pub search: Option<String>,
    /// Only this insert style.
    pub style: Option<InsertStyle>,
    /// Sorted by.
    pub sort: DesignSort,
    /// Which way.
    pub order: SortOrder,
}

/// Designs a user saves from the editor (CLAUDE.md §6, §7). A design holds copies of its
/// products' outlines, so a capture corrected later doesn't change it.
pub trait DesignService: Send + Sync {
    /// Checks the spec (`insert_core`'s rules: `DESIGN_INVALID` names what is wrong) and
    /// saves it at version 1.
    fn create<'a>(
        &'a self,
        request: NewDesignRequest<'a>,
    ) -> BoxFuture<'a, Result<SavedDesign, AppError>>;

    /// The caller's design. `DESIGN_NOT_FOUND` for anyone else's.
    fn get<'a>(
        &'a self,
        request: (&'a AuthUser, Uuid),
    ) -> BoxFuture<'a, Result<SavedDesign, AppError>>;

    /// Saves a change made from `change.from_version`: the version goes up by one.
    /// `DESIGN_VERSION_CONFLICT` when the design was saved meanwhile (another tab). A change
    /// that changes nothing keeps the version, so an autosave of the same state is free.
    fn update<'a>(
        &'a self,
        request: DesignUpdate<'a>,
    ) -> BoxFuture<'a, Result<SavedDesign, AppError>>;

    /// A page of the caller's designs, filtered and sorted.
    fn list<'a>(
        &'a self,
        request: DesignListRequest<'a>,
    ) -> BoxFuture<'a, Result<Page<DesignSummary>, AppError>>;
}
