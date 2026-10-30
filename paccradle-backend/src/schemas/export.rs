//! Export bodies (M1-D7): files to make, an export with its download links, and a design's
//! export history.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use super::common::page_request;
use crate::error::{AppError, ErrorCode, ErrorReport};
use crate::locales::Locale;
use crate::models::entitlement::PaidWith;
use crate::models::export::{ExportFormat, ExportStatus, ExportSummary, ExportView};
use crate::models::page::{PageRequest, SortOrder};
use crate::models::paper::Paper;

/// `POST /designs/{design_id}/exports`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateExportRequest {
    /// Any of `svg`, `pdf`, `dxf` (production files, paid) and `png`, `test_print` (free).
    pub formats: Vec<ExportFormat>,
    /// `a4` or `letter`: the test print's paper, needed with `test_print`.
    #[serde(default)]
    pub paper: Option<Paper>,
    /// The design version the user is looking at; another one is `DESIGN_VERSION_CONFLICT`.
    #[serde(default)]
    pub version: Option<i32>,
}

/// What an export request asks for, checked.
#[derive(Debug, Clone)]
pub struct ExportAsk {
    /// The formats, each once, in the order asked.
    pub formats: Vec<ExportFormat>,
    /// The test print's paper; none when there is no test print.
    pub paper: Option<Paper>,
    /// The version the user is looking at.
    pub version: Option<i32>,
}

impl CreateExportRequest {
    /// The request, checked: at least one format, and a paper for a test print.
    pub fn ask(self) -> Result<ExportAsk, AppError> {
        let mut formats = Vec::with_capacity(self.formats.len());
        for format in self.formats {
            if !formats.contains(&format) {
                formats.push(format);
            }
        }
        if formats.is_empty() {
            return Err(AppError::new(ErrorCode::InvalidRequest)
                .with_meta("field", "formats")
                .with_meta("allowed", "svg, pdf, dxf, png, test_print"));
        }
        let test_print = formats.contains(&ExportFormat::TestPrint);
        if test_print && self.paper.is_none() {
            return Err(AppError::new(ErrorCode::InvalidRequest)
                .with_meta("field", "paper")
                .with_meta("allowed", "a4 or letter, for test_print"));
        }
        Ok(ExportAsk {
            formats,
            paper: self.paper.filter(|_| test_print),
            version: self.version,
        })
    }
}

/// `GET /designs/{design_id}/exports`: which page.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExportListParams {
    /// From 1; default 1.
    pub page: Option<u32>,
    /// 1 to 100; default 20.
    pub page_size: Option<u32>,
    /// Only exports in this state.
    pub status: Option<ExportStatus>,
    /// Sorted by when they were asked for: only `created_at`.
    pub sort: Option<ExportSort>,
    /// `desc` (newest first, the default) or `asc`.
    pub order: Option<SortOrder>,
}

/// What a design's export history is sorted by: when each was asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportSort {
    /// When it was asked for.
    CreatedAt,
}

impl ExportListParams {
    /// The page, filter and order, with their defaults.
    pub fn filters(self) -> Result<(PageRequest, Option<ExportStatus>, SortOrder), AppError> {
        Ok((
            page_request((self.page, self.page_size))?,
            self.status,
            self.order.unwrap_or_default(),
        ))
    }
}

/// An export: `POST` (202), `GET /exports/{export_id}`.
#[derive(Debug, Serialize)]
pub struct ExportResponse {
    /// The export's id.
    pub export_id: Uuid,
    /// The design.
    pub design_id: Uuid,
    /// The design's version the files are made from.
    pub design_version: i32,
    /// `pending` (poll again), `done` (see `files`) or `failed` (see `error`).
    pub status: ExportStatus,
    /// What was asked for.
    pub formats: Vec<ExportFormat>,
    /// The test print's paper.
    pub paper: Option<Paper>,
    /// How the production files were covered: `free` (none asked for), `unlocked`, `pro`,
    /// `credit`.
    pub paid_with: PaidWith,
    /// Once done: each file with a link that works for 5 minutes (fetch the export again
    /// for new ones).
    pub files: Vec<ExportFileResponse>,
    /// Why it failed, in the caller's language. Asking again costs nothing.
    pub error: Option<ExportErrorResponse>,
    /// When it was asked for.
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

/// A made file and its download link.
#[derive(Debug, Serialize)]
pub struct ExportFileResponse {
    /// Which format.
    pub format: ExportFormat,
    /// The name it downloads as.
    pub file_name: String,
    /// Its size.
    pub bytes: u64,
    /// The signed link (it downloads as `file_name`).
    pub url: String,
    /// When the link stops working.
    #[serde(with = "time::serde::rfc3339")]
    pub url_expires_at: OffsetDateTime,
}

/// A failed export's error.
#[derive(Debug, Serialize)]
pub struct ExportErrorResponse {
    /// E.g. `EXPORT_FAILED`.
    pub code: &'static str,
    /// In the caller's language.
    pub message: String,
}

impl ExportResponse {
    /// The export as the caller sees it, its error in `locale`.
    pub fn new((view, locale): (ExportView, Locale)) -> Self {
        let export = view.export;
        let error = export.error_code.as_deref().map(|code| {
            let code = ErrorCode::from_code(code).unwrap_or(ErrorCode::ExportFailed);
            let report = ErrorReport {
                code,
                meta: serde_json::Map::new(),
            };
            ExportErrorResponse {
                code: code.as_str(),
                message: locale.error_message(&report),
            }
        });
        Self {
            export_id: export.id,
            design_id: export.design_id,
            design_version: export.design_version,
            status: export.status,
            formats: export.formats,
            paper: export.paper,
            paid_with: export.paid_with,
            files: view
                .links
                .into_iter()
                .map(|link| ExportFileResponse {
                    format: link.file.format,
                    file_name: link.file.file_name,
                    bytes: link.file.bytes,
                    url: link.url,
                    url_expires_at: link.expires_at,
                })
                .collect(),
            error,
            created_at: export.created_at,
        }
    }
}

/// An export in a design's history: open it with `GET /exports/{export_id}` for its files.
#[derive(Debug, Serialize)]
pub struct ExportSummaryResponse {
    /// The export's id.
    pub export_id: Uuid,
    /// The design's version.
    pub design_version: i32,
    /// `pending`, `done` or `failed`.
    pub status: ExportStatus,
    /// What was asked for.
    pub formats: Vec<ExportFormat>,
    /// How the production files were covered.
    pub paid_with: PaidWith,
    /// When it was asked for.
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

impl From<ExportSummary> for ExportSummaryResponse {
    fn from(export: ExportSummary) -> Self {
        Self {
            export_id: export.id,
            design_version: export.design_version,
            status: export.status,
            formats: export.formats,
            paid_with: export.paid_with,
            created_at: export.created_at,
        }
    }
}
