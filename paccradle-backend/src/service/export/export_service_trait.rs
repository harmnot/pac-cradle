use uuid::Uuid;

use crate::error::AppError;
use crate::models::export::{
    ExportFormat, ExportStatus, ExportSummary, ExportView, RunExportPayload,
};
use crate::models::page::{Page, PageRequest, SortOrder};
use crate::models::paper::Paper;
use crate::models::session::AuthUser;
use crate::shared::future::BoxFuture;

/// Files to make from one of the caller's designs.
#[derive(Debug, Clone)]
pub struct ExportRequest<'a> {
    /// Who asks.
    pub user: &'a AuthUser,
    /// Which design.
    pub design_id: Uuid,
    /// What to make, each once, at least one.
    pub formats: Vec<ExportFormat>,
    /// The test print's paper (needed for it).
    pub paper: Option<Paper>,
    /// The version the user is looking at; another one is a conflict.
    pub version: Option<i32>,
}

/// A page of one of the caller's designs' exports.
#[derive(Debug, Clone)]
pub struct ExportListRequest<'a> {
    /// Who asks.
    pub user: &'a AuthUser,
    /// Which design.
    pub design_id: Uuid,
    /// Which page.
    pub page: PageRequest,
    /// Only exports in this state.
    pub status: Option<ExportStatus>,
    /// By when they were asked for, which way.
    pub order: SortOrder,
}

/// Exports (CLAUDE.md §9.3).
pub trait ExportService: Send + Sync {
    /// Asks for files from the design's current version. Needs an account
    /// (`AUTH_ACCOUNT_REQUIRED` for a guest, DEC-14) and a design that can be made
    /// (`DESIGN_NOT_BUILDABLE` otherwise, before anything is charged). The PNG preview and
    /// the test print are free; SVG, PDF and DXF need the design covered
    /// (`BillingService::entitle_export`, else `EXPORT_PAYMENT_REQUIRED`). The export keeps
    /// a copy of the spec and its job is queued in the same transaction as any credit spent.
    /// `RATE_LIMITED` after [`super::EXPORTS_PER_HOUR`] an hour.
    fn request<'a>(
        &'a self,
        request: ExportRequest<'a>,
    ) -> BoxFuture<'a, Result<ExportView, AppError>>;

    /// The caller's export, with 5-minute download links once it is done.
    /// `EXPORT_NOT_FOUND` for anyone else's.
    fn get<'a>(
        &'a self,
        request: (&'a AuthUser, Uuid),
    ) -> BoxFuture<'a, Result<ExportView, AppError>>;

    /// A page of one of the caller's designs' exports (`DESIGN_NOT_FOUND` for anyone
    /// else's design).
    fn list<'a>(
        &'a self,
        request: ExportListRequest<'a>,
    ) -> BoxFuture<'a, Result<Page<ExportSummary>, AppError>>;

    /// Makes a pending export's files and stores them (the `run_export` job). Storage and
    /// database outages are errors for the job to retry; files that can't be made fail it.
    fn run(&self, export: RunExportPayload) -> BoxFuture<'_, Result<(), AppError>>;

    /// Marks an export whose job failed for good as failed (`EXPORT_FAILED`).
    fn give_up(&self, export: RunExportPayload) -> BoxFuture<'_, Result<(), AppError>>;
}
