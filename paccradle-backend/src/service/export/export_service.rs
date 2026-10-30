use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::{Duration, Instant};

use insert_core::GenerateError;
use insert_core::export::PngError;
use time::OffsetDateTime;
use uuid::Uuid;

use super::{ExportListRequest, ExportRequest, ExportService};
use crate::error::{AppError, ErrorCode};
use crate::models::entitlement::PaidWith;
use crate::models::export::{
    Export, ExportFile, ExportFormat, ExportLink, ExportListQuery, ExportStatus, ExportSummary,
    ExportView, NewExport, RunExport, RunExportPayload,
};
use crate::models::job::NewJob;
use crate::models::page::Page;
use crate::models::paper::Paper;
use crate::models::session::AuthUser;
use crate::models::user::AccountRole;
use crate::repository::database::Database;
use crate::repository::export::ExportRepository;
use crate::service::billing::{BillingService, ExportEntitlement};
use crate::service::design::DesignService;
use crate::service::job::JobService;
use crate::shared::future::BoxFuture;
use crate::shared::id::new_id;
use crate::shared::rate_limiter::RateLimiter;
use crate::third_party::storage::{NewObject, ObjectStorage};

/// How long a download link works: paid files are reached only through links that expire
/// in minutes (CLAUDE.md §8.5).
pub const EXPORT_URL_TTL: Duration = Duration::from_secs(5 * 60);
/// Exports one user may ask for per hour (free ones too: each makes files on the server).
pub const EXPORTS_PER_HOUR: u32 = 60;
/// Longest part of a file name taken from the design's name.
const MAX_SLUG_CHARS: usize = 48;

/// What [`DefaultExportService`] works with.
pub struct ExportProviders {
    /// Connections and transactions.
    pub db: Arc<dyn Database>,
    /// `exports`.
    pub exports: Arc<dyn ExportRepository>,
    /// The caller's designs.
    pub designs: Arc<dyn DesignService>,
    /// What the caller has paid for.
    pub billing: Arc<dyn BillingService>,
    /// The job queue (files are made by a job).
    pub jobs: Arc<dyn JobService>,
    /// Where files are kept.
    pub storage: Arc<dyn ObjectStorage>,
}

/// [`ExportService`] on Postgres, object storage, the job queue and `insert_core`'s writers.
pub struct DefaultExportService {
    parts: ExportProviders,
    requests: RateLimiter<Uuid>,
}

/// A file made, before it is stored.
struct MadeFile {
    format: ExportFormat,
    file_name: String,
    content_type: &'static str,
    bytes: Vec<u8>,
}

/// Why an export's files couldn't be made.
#[derive(Debug, thiserror::Error)]
enum MakeError {
    #[error(transparent)]
    Generate(#[from] GenerateError),
    #[error(transparent)]
    Png(#[from] PngError),
}

impl DefaultExportService {
    /// A service on `parts`.
    pub fn new(parts: ExportProviders) -> Self {
        let per_hour = NonZeroU32::new(EXPORTS_PER_HOUR).unwrap_or(NonZeroU32::MIN);
        Self {
            parts,
            requests: RateLimiter::with_window((per_hour, Duration::from_secs(60 * 60))),
        }
    }

    /// The export with links to its files, once it is done.
    fn view(&self, export: Export) -> Result<ExportView, AppError> {
        if export.status != ExportStatus::Done {
            return Ok(ExportView {
                export,
                links: Vec::new(),
            });
        }
        let expires_at = OffsetDateTime::now_utc() + EXPORT_URL_TTL;
        let links = export
            .files
            .iter()
            .map(|file| {
                let url = self
                    .parts
                    .storage
                    .signed_url((&file.storage_key, EXPORT_URL_TTL))?;
                Ok(ExportLink {
                    file: file.clone(),
                    url,
                    expires_at,
                })
            })
            .collect::<Result<_, AppError>>()?;
        Ok(ExportView { export, links })
    }

    /// Records why an export's files couldn't be made.
    async fn fail(&self, (id, code): (Uuid, ErrorCode)) -> Result<(), AppError> {
        let mut conn = self.parts.db.connection().await?;
        self.parts
            .exports
            .fail(&mut conn, (id, code.as_str()))
            .await?;
        Ok(())
    }
}

impl ExportService for DefaultExportService {
    fn request<'a>(
        &'a self,
        request: ExportRequest<'a>,
    ) -> BoxFuture<'a, Result<ExportView, AppError>> {
        Box::pin(async move {
            let ExportRequest {
                user,
                design_id,
                formats,
                paper,
                version,
            } = request;
            if user.role == AccountRole::Guest {
                return Err(AppError::new(ErrorCode::AuthAccountRequired));
            }
            if let Err(wait) = self.requests.check(user.user_id, Instant::now()) {
                return Err(AppError::new(ErrorCode::RateLimited)
                    .with_meta("retry_after_seconds", wait.as_secs().max(1)));
            }
            let design = self.parts.designs.get((user, design_id)).await?;
            if let Some(version) = version
                && version != design.version
            {
                return Err(AppError::new(ErrorCode::DesignVersionConflict)
                    .with_meta("current_version", design.version));
            }
            // Before anything is charged: a design that can't be made costs nothing.
            let spec = design.spec.clone();
            blocking(move || spec.generate())
                .await?
                .map_err(not_buildable)?;

            let mut tx = self.parts.db.begin().await?;
            let paid_with = match formats.iter().any(|format| format.is_paid()) {
                true => {
                    let entitlement = ExportEntitlement {
                        user,
                        design: &design,
                    };
                    self.parts
                        .billing
                        .entitle_export((&mut tx, entitlement))
                        .await?
                }
                false => PaidWith::Free,
            };
            let export = NewExport {
                id: new_id(),
                design_id,
                design_version: design.version,
                spec: design.spec,
                design_name: design.name,
                formats,
                paper,
                paid_with,
            };
            self.parts.exports.insert(&mut tx, &export).await?;
            let job = NewJob::of::<RunExport>(&RunExportPayload {
                export_id: export.id,
            })?;
            self.parts.jobs.enqueue(&mut tx, &job).await?;
            let stored = self
                .parts
                .exports
                .find(&mut tx, export.id)
                .await?
                .ok_or_else(|| AppError::new(ErrorCode::Internal))?;
            tx.commit().await?;
            self.view(stored)
        })
    }

    fn get<'a>(
        &'a self,
        (user, id): (&'a AuthUser, Uuid),
    ) -> BoxFuture<'a, Result<ExportView, AppError>> {
        Box::pin(async move {
            let mut conn = self.parts.db.connection().await?;
            let export = self.parts.exports.find(&mut conn, id).await?;
            match export {
                Some(export) if export.user_id == user.user_id => self.view(export),
                _ => Err(AppError::new(ErrorCode::ExportNotFound)),
            }
        })
    }

    fn list<'a>(
        &'a self,
        request: ExportListRequest<'a>,
    ) -> BoxFuture<'a, Result<Page<ExportSummary>, AppError>> {
        Box::pin(async move {
            // The design must be the caller's.
            self.parts
                .designs
                .get((request.user, request.design_id))
                .await?;
            let query = ExportListQuery {
                design_id: request.design_id,
                page: request.page,
                status: request.status,
                order: request.order,
            };
            let mut conn = self.parts.db.connection().await?;
            let total = self.parts.exports.count(&mut conn, &query).await?;
            let items = self.parts.exports.list(&mut conn, &query).await?;
            Ok(Page {
                items,
                request: query.page,
                total,
            })
        })
    }

    fn run(&self, payload: RunExportPayload) -> BoxFuture<'_, Result<(), AppError>> {
        Box::pin(async move {
            let export = {
                let mut conn = self.parts.db.connection().await?;
                self.parts
                    .exports
                    .find(&mut conn, payload.export_id)
                    .await?
            };
            let Some(export) = export else {
                return Ok(());
            };
            if export.status != ExportStatus::Pending {
                return Ok(());
            }
            let id = export.id;
            let made = match blocking(move || make_files(&export)).await? {
                Ok(made) => made,
                Err(error) => {
                    tracing::error!(export_id = %id, %error, "an export's files can't be made");
                    let code = match error {
                        MakeError::Generate(GenerateError::Geometry(_)) | MakeError::Png(_) => {
                            ErrorCode::ExportFailed
                        }
                        MakeError::Generate(_) => ErrorCode::DesignNotBuildable,
                    };
                    return self.fail((id, code)).await;
                }
            };
            let mut files = Vec::with_capacity(made.len());
            for file in made {
                let storage_key = format!("exports/{id}/{}", file.file_name);
                let bytes = file.bytes.len() as u64;
                self.parts
                    .storage
                    .put(NewObject {
                        key: storage_key.clone(),
                        bytes: file.bytes,
                        content_type: file.content_type,
                        content_disposition: Some(format!(
                            "attachment; filename=\"{}\"",
                            file.file_name
                        )),
                    })
                    .await?;
                files.push(ExportFile {
                    format: file.format,
                    file_name: file.file_name,
                    storage_key,
                    bytes,
                });
            }
            let mut conn = self.parts.db.connection().await?;
            self.parts.exports.finish(&mut conn, (id, &files)).await?;
            Ok(())
        })
    }

    fn give_up(&self, payload: RunExportPayload) -> BoxFuture<'_, Result<(), AppError>> {
        Box::pin(async move {
            self.fail((payload.export_id, ErrorCode::ExportFailed))
                .await
        })
    }
}

/// Every file the export asks for, from its copy of the spec.
fn make_files(export: &Export) -> Result<Vec<MadeFile>, MakeError> {
    let packaging = export.spec.generate()?;
    let mut made = Vec::with_capacity(export.formats.len());
    for &format in &export.formats {
        let (bytes, content_type) = match format {
            ExportFormat::Svg => (packaging.to_svg().into_bytes(), "image/svg+xml"),
            ExportFormat::Pdf => (packaging.to_pdf(), "application/pdf"),
            ExportFormat::Dxf => (packaging.to_dxf().into_bytes(), "application/dxf"),
            ExportFormat::Png => (packaging.to_png(&export.spec)?, "image/png"),
            ExportFormat::TestPrint => {
                let paper = match export.paper.unwrap_or(Paper::A4) {
                    Paper::A4 => insert_core::export::Paper::A4,
                    Paper::Letter => insert_core::export::Paper::Letter,
                };
                (packaging.to_test_print(paper), "application/pdf")
            }
        };
        made.push(MadeFile {
            format,
            file_name: file_name((export, format)),
            content_type,
            bytes,
        });
    }
    Ok(made)
}

/// The name a file downloads as: `paccradle-<design name>-v<version>`, then what it is, e.g.
/// `paccradle-perfume-set-v3.svg` or `paccradle-perfume-set-v3-test-print-a4.pdf`.
pub fn file_name((export, format): (&Export, ExportFormat)) -> String {
    let stem = format!(
        "paccradle-{}-v{}",
        slug(&export.design_name),
        export.design_version
    );
    match format {
        ExportFormat::Svg => format!("{stem}.svg"),
        ExportFormat::Pdf => format!("{stem}.pdf"),
        ExportFormat::Dxf => format!("{stem}.dxf"),
        ExportFormat::Png => format!("{stem}-preview.png"),
        ExportFormat::TestPrint => {
            let paper = export.paper.unwrap_or(Paper::A4);
            let paper = match paper {
                Paper::A4 => "a4",
                Paper::Letter => "letter",
            };
            format!("{stem}-test-print-{paper}.pdf")
        }
    }
}

/// A design name as plain ASCII for a file name: letters and digits in lower case, anything
/// else a single hyphen; `design` when nothing is left.
fn slug(name: &str) -> String {
    let mut slug = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c.to_ascii_lowercase());
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let slug: String = slug
        .trim_matches('-')
        .chars()
        .take(MAX_SLUG_CHARS)
        .collect();
    match slug.trim_end_matches('-') {
        "" => "design".to_owned(),
        slug => slug.to_owned(),
    }
}

/// `DESIGN_NOT_BUILDABLE` with `insert_core`'s words, as the editor shows them; a geometry
/// error is ours.
fn not_buildable(error: GenerateError) -> AppError {
    match error {
        GenerateError::Geometry(_) => AppError::new(ErrorCode::Internal).with_source(error),
        _ => AppError::new(ErrorCode::DesignNotBuildable).with_meta("detail", error.to_string()),
    }
}

/// Runs `work` off the async threads.
async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> T + Send + 'static,
) -> Result<T, AppError> {
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|error| AppError::new(ErrorCode::Internal).with_source(error))
}
