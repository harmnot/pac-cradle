use serde_json::Value;
use sqlx::PgConnection;
use time::OffsetDateTime;
use uuid::Uuid;

use super::ExportRepository;
use crate::error::{AppError, ErrorCode};
use crate::models::TextEnum;
use crate::models::entitlement::PaidWith;
use crate::models::export::{
    Export, ExportFile, ExportFormat, ExportListQuery, ExportStatus, ExportSummary, NewExport,
};
use crate::models::paper::Paper;
use crate::shared::future::BoxFuture;

/// [`ExportRepository`] on Postgres.
#[derive(Default)]
pub struct PgExportRepository;

impl PgExportRepository {
    /// The repository.
    pub fn new() -> Self {
        Self
    }
}

struct ExportRow {
    id: Uuid,
    design_id: Uuid,
    user_id: Uuid,
    design_version: i32,
    spec: Value,
    design_name: String,
    formats: Vec<String>,
    paper: Option<String>,
    paid_with: String,
    status: String,
    files: Value,
    error_code: Option<String>,
    created_at: OffsetDateTime,
}

impl TryFrom<ExportRow> for Export {
    type Error = AppError;

    fn try_from(row: ExportRow) -> Result<Self, AppError> {
        Ok(Self {
            id: row.id,
            design_id: row.design_id,
            user_id: row.user_id,
            design_version: row.design_version,
            spec: serde_json::from_value(row.spec).map_err(internal)?,
            design_name: row.design_name,
            formats: formats(&row.formats)?,
            paper: row.paper.as_deref().map(Paper::from_text).transpose()?,
            paid_with: PaidWith::from_text(&row.paid_with)?,
            status: ExportStatus::from_text(&row.status)?,
            files: serde_json::from_value(row.files).map_err(internal)?,
            error_code: row.error_code,
            created_at: row.created_at,
        })
    }
}

struct SummaryRow {
    id: Uuid,
    design_version: i32,
    formats: Vec<String>,
    paid_with: String,
    status: String,
    created_at: OffsetDateTime,
}

impl TryFrom<SummaryRow> for ExportSummary {
    type Error = AppError;

    fn try_from(row: SummaryRow) -> Result<Self, AppError> {
        Ok(Self {
            id: row.id,
            design_version: row.design_version,
            formats: formats(&row.formats)?,
            paid_with: PaidWith::from_text(&row.paid_with)?,
            status: ExportStatus::from_text(&row.status)?,
            created_at: row.created_at,
        })
    }
}

impl ExportRepository for PgExportRepository {
    fn insert<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        export: &'a NewExport,
    ) -> BoxFuture<'a, Result<(), AppError>> {
        Box::pin(async move {
            let spec = serde_json::to_value(&export.spec).map_err(internal)?;
            let formats: Vec<&str> = export.formats.iter().map(|f| f.as_str()).collect();
            sqlx::query!(
                r#"
                INSERT INTO exports
                    (id, design_id, design_version, spec, design_name, formats, paper, paid_with)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                "#,
                export.id,
                export.design_id,
                export.design_version,
                spec,
                export.design_name,
                &formats as &[&str],
                export.paper.map(TextEnum::as_str),
                export.paid_with.as_str(),
            )
            .execute(conn)
            .await?;
            Ok(())
        })
    }

    fn find<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        id: Uuid,
    ) -> BoxFuture<'a, Result<Option<Export>, AppError>> {
        Box::pin(async move {
            let row = sqlx::query_as!(
                ExportRow,
                r#"
                SELECT exports.id, exports.design_id, designs.user_id, exports.design_version,
                       exports.spec, exports.design_name, exports.formats, exports.paper,
                       exports.paid_with, exports.status, exports.files, exports.error_code,
                       exports.created_at
                FROM exports JOIN designs ON designs.id = exports.design_id
                WHERE exports.id = $1
                "#,
                id,
            )
            .fetch_optional(conn)
            .await?;
            row.map(Export::try_from).transpose()
        })
    }

    fn finish<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        (id, files): (Uuid, &'a [ExportFile]),
    ) -> BoxFuture<'a, Result<bool, AppError>> {
        Box::pin(async move {
            let files = serde_json::to_value(files).map_err(internal)?;
            let result = sqlx::query!(
                "UPDATE exports SET status = 'done', files = $2 WHERE id = $1 AND status = 'pending'",
                id,
                files,
            )
            .execute(conn)
            .await?;
            Ok(result.rows_affected() == 1)
        })
    }

    fn fail<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        (id, code): (Uuid, &'a str),
    ) -> BoxFuture<'a, Result<bool, AppError>> {
        Box::pin(async move {
            let result = sqlx::query!(
                r#"
                UPDATE exports SET status = 'failed', error_code = $2
                WHERE id = $1 AND status = 'pending'
                "#,
                id,
                code,
            )
            .execute(conn)
            .await?;
            Ok(result.rows_affected() == 1)
        })
    }

    fn list<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        query: &'a ExportListQuery,
    ) -> BoxFuture<'a, Result<Vec<ExportSummary>, AppError>> {
        Box::pin(async move {
            let rows = sqlx::query_as!(
                SummaryRow,
                r#"
                SELECT id, design_version, formats, paid_with, status, created_at
                FROM exports
                WHERE design_id = $1 AND ($2::text IS NULL OR status = $2)
                ORDER BY
                    CASE WHEN $3 = 'asc' THEN created_at END ASC,
                    CASE WHEN $3 = 'desc' THEN created_at END DESC,
                    id
                LIMIT $4 OFFSET $5
                "#,
                query.design_id,
                query.status.map(TextEnum::as_str),
                query.order.as_str(),
                query.page.limit(),
                query.page.offset(),
            )
            .fetch_all(conn)
            .await?;
            rows.into_iter().map(ExportSummary::try_from).collect()
        })
    }

    fn count<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        query: &'a ExportListQuery,
    ) -> BoxFuture<'a, Result<u64, AppError>> {
        Box::pin(async move {
            let count = sqlx::query_scalar!(
                r#"
                SELECT count(*) AS "count!" FROM exports
                WHERE design_id = $1 AND ($2::text IS NULL OR status = $2)
                "#,
                query.design_id,
                query.status.map(TextEnum::as_str),
            )
            .fetch_one(conn)
            .await?;
            Ok(u64::try_from(count).unwrap_or_default())
        })
    }
}

fn formats(stored: &[String]) -> Result<Vec<ExportFormat>, AppError> {
    stored
        .iter()
        .map(|text| ExportFormat::from_text(text))
        .collect()
}

fn internal(error: serde_json::Error) -> AppError {
    AppError::new(ErrorCode::Internal).with_source(error)
}
