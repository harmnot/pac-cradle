//! Exported files (the `exports` table, CLAUDE.md §6.6, §9.3).

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use super::TextEnum;
use super::entitlement::PaidWith;
use super::job::JobKind;
use super::page::{PageRequest, SortOrder};
use super::paper::Paper;

/// A file an export can make.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    /// Production SVG: cut and crease layers (paid).
    Svg,
    /// Production PDF drawing with labels and dimensions (paid).
    Pdf,
    /// Production DXF R12 for cutting tables (paid).
    Dxf,
    /// The watermarked preview picture (free).
    Png,
    /// The watermarked print-at-home test print, tiled over A4 or Letter (free).
    TestPrint,
}

impl ExportFormat {
    /// Whether it is a production file, which needs the design unlocked.
    pub const fn is_paid(self) -> bool {
        matches!(self, Self::Svg | Self::Pdf | Self::Dxf)
    }
}

impl TextEnum for ExportFormat {
    const ALL: &'static [Self] = &[Self::Svg, Self::Pdf, Self::Dxf, Self::Png, Self::TestPrint];

    fn as_str(self) -> &'static str {
        match self {
            Self::Svg => "svg",
            Self::Pdf => "pdf",
            Self::Dxf => "dxf",
            Self::Png => "png",
            Self::TestPrint => "test_print",
        }
    }
}

/// `exports.status`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportStatus {
    /// Its job hasn't made the files yet.
    Pending,
    /// Being made (not used yet: a job goes from pending to done or failed).
    Running,
    /// The files are stored.
    Done,
    /// The files couldn't be made (`error_code`).
    Failed,
}

impl TextEnum for ExportStatus {
    const ALL: &'static [Self] = &[Self::Pending, Self::Running, Self::Done, Self::Failed];

    fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Done => "done",
            Self::Failed => "failed",
        }
    }
}

/// A file an export made, as `exports.files` stores it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportFile {
    /// Which format.
    pub format: ExportFormat,
    /// The name it downloads as, e.g. `paccradle-perfume-set-v3.svg`.
    pub file_name: String,
    /// Where it is stored.
    pub storage_key: String,
    /// Its size.
    pub bytes: u64,
}

/// An export to add, with its job.
#[derive(Debug, Clone)]
pub struct NewExport {
    /// The id it will have.
    pub id: Uuid,
    /// The design.
    pub design_id: Uuid,
    /// The design's version it is made from.
    pub design_version: i32,
    /// That version's spec.
    pub spec: insert_core::Design,
    /// The design's name then.
    pub design_name: String,
    /// What to make, each once.
    pub formats: Vec<ExportFormat>,
    /// The test print's paper.
    pub paper: Option<Paper>,
    /// How the paid formats were covered.
    pub paid_with: PaidWith,
}

/// An export as stored.
#[derive(Debug, Clone, PartialEq)]
pub struct Export {
    /// The export's id.
    pub id: Uuid,
    /// The design.
    pub design_id: Uuid,
    /// Whose design it is.
    pub user_id: Uuid,
    /// The design's version it was made from.
    pub design_version: i32,
    /// That version's spec.
    pub spec: insert_core::Design,
    /// The design's name then.
    pub design_name: String,
    /// What was asked for.
    pub formats: Vec<ExportFormat>,
    /// The test print's paper.
    pub paper: Option<Paper>,
    /// How the paid formats were covered.
    pub paid_with: PaidWith,
    /// Where its job is.
    pub status: ExportStatus,
    /// The files, once made.
    pub files: Vec<ExportFile>,
    /// Why it failed.
    pub error_code: Option<String>,
    /// When it was asked for.
    pub created_at: OffsetDateTime,
}

/// A file with a link that works for a few minutes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportLink {
    /// The file.
    pub file: ExportFile,
    /// The signed URL.
    pub url: String,
    /// When it stops working.
    pub expires_at: OffsetDateTime,
}

/// An export, with download links once its files are made.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportView {
    /// The export.
    pub export: Export,
    /// A link per file; empty until it is done.
    pub links: Vec<ExportLink>,
}

/// An export in a design's history: no spec copy.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportSummary {
    /// The export's id.
    pub id: Uuid,
    /// The design's version it was made from.
    pub design_version: i32,
    /// What was asked for.
    pub formats: Vec<ExportFormat>,
    /// How the paid formats were covered.
    pub paid_with: PaidWith,
    /// Where its job is.
    pub status: ExportStatus,
    /// When it was asked for.
    pub created_at: OffsetDateTime,
}

/// A page of one design's exports.
#[derive(Debug, Clone)]
pub struct ExportListQuery {
    /// The design.
    pub design_id: Uuid,
    /// Which page.
    pub page: PageRequest,
    /// Only exports in this state.
    pub status: Option<ExportStatus>,
    /// By when they were asked for, which way.
    pub order: SortOrder,
}

/// Makes an export's files and stores them (M1-D7).
pub struct RunExport;

/// The export to make.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunExportPayload {
    /// The export.
    pub export_id: Uuid,
}

impl JobKind for RunExport {
    const NAME: &'static str = "run_export";
    type Payload = RunExportPayload;
}
