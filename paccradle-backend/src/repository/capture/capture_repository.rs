use serde_json::{Map, Value};
use sqlx::PgConnection;
use time::OffsetDateTime;
use uuid::Uuid;

use super::CaptureRepository;
use crate::error::{AppError, ErrorCode};
use crate::models::TextEnum;
use crate::models::capture::{
    Capture, CaptureStatus, DuePhoto, NewCapture, StoredError, VisionLevel,
};
use crate::shared::future::BoxFuture;

/// [`CaptureRepository`] on Postgres.
#[derive(Default)]
pub struct PgCaptureRepository;

impl PgCaptureRepository {
    /// The repository.
    pub fn new() -> Self {
        Self
    }
}

struct CaptureRow {
    id: Uuid,
    user_id: Uuid,
    status: String,
    input: Value,
    run: i32,
    result: Option<Value>,
    confidence: Option<f64>,
    level_used: Option<String>,
    error_code: Option<String>,
    error_meta: Value,
    storage_key: Option<String>,
    created_at: OffsetDateTime,
    purge_after: OffsetDateTime,
}

impl TryFrom<CaptureRow> for Capture {
    type Error = AppError;

    fn try_from(row: CaptureRow) -> Result<Self, AppError> {
        let error = row.error_code.map(|code| StoredError {
            code,
            meta: match row.error_meta {
                Value::Object(meta) => meta,
                _ => Map::new(),
            },
        });
        Ok(Self {
            id: row.id,
            user_id: row.user_id,
            status: CaptureStatus::from_text(&row.status)?,
            settings: serde_json::from_value(row.input).map_err(internal)?,
            run: row.run,
            result: row.result,
            confidence: row.confidence,
            level_used: row
                .level_used
                .as_deref()
                .map(VisionLevel::from_text)
                .transpose()?,
            error,
            storage_key: row.storage_key,
            created_at: row.created_at,
            purge_after: row.purge_after,
        })
    }
}

impl CaptureRepository for PgCaptureRepository {
    fn insert<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        capture: &'a NewCapture,
    ) -> BoxFuture<'a, Result<Capture, AppError>> {
        Box::pin(async move {
            let input = serde_json::to_value(capture.settings).map_err(internal)?;
            let exif = serde_json::to_value(&capture.exif).map_err(internal)?;
            let row = sqlx::query_as!(
                CaptureRow,
                r#"
                INSERT INTO captures (id, user_id, mode, paper, input, exif, storage_key, purge_after)
                VALUES ($1, $2, $3, $4, $5, $6, $7, now() + make_interval(secs => $8))
                RETURNING id, user_id, status, input, run, result, confidence, level_used,
                          error_code, error_meta, storage_key, created_at, purge_after
                "#,
                capture.id,
                capture.user_id,
                capture.settings.mode.as_str(),
                capture.settings.paper.as_str(),
                input,
                exif,
                capture.storage_key,
                capture.photo_retention.as_secs_f64(),
            )
            .fetch_one(conn)
            .await?;
            Capture::try_from(row)
        })
    }

    fn find<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        id: Uuid,
    ) -> BoxFuture<'a, Result<Option<Capture>, AppError>> {
        Box::pin(async move {
            let row = sqlx::query_as!(
                CaptureRow,
                r#"
                SELECT id, user_id, status, input, run, result, confidence, level_used,
                       error_code, error_meta, storage_key, created_at, purge_after
                FROM captures WHERE id = $1
                "#,
                id,
            )
            .fetch_optional(conn)
            .await?;
            row.map(Capture::try_from).transpose()
        })
    }

    fn lock<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        id: Uuid,
    ) -> BoxFuture<'a, Result<Option<Capture>, AppError>> {
        Box::pin(async move {
            let row = sqlx::query_as!(
                CaptureRow,
                r#"
                SELECT id, user_id, status, input, run, result, confidence, level_used,
                       error_code, error_meta, storage_key, created_at, purge_after
                FROM captures WHERE id = $1
                FOR UPDATE
                "#,
                id,
            )
            .fetch_optional(conn)
            .await?;
            row.map(Capture::try_from).transpose()
        })
    }

    fn save<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        capture: &'a Capture,
    ) -> BoxFuture<'a, Result<(), AppError>> {
        Box::pin(async move {
            let input = serde_json::to_value(capture.settings).map_err(internal)?;
            let (error_code, error_meta) = match &capture.error {
                Some(error) => (Some(error.code.as_str()), Value::Object(error.meta.clone())),
                None => (None, Value::Object(Map::new())),
            };
            sqlx::query!(
                r#"
                UPDATE captures
                SET status = $2, input = $3, run = $4, result = $5, confidence = $6,
                    level_used = $7, error_code = $8, error_meta = $9
                WHERE id = $1
                "#,
                capture.id,
                capture.status.as_str(),
                input,
                capture.run,
                capture.result,
                capture.confidence,
                capture.level_used.map(TextEnum::as_str),
                error_code,
                error_meta,
            )
            .execute(conn)
            .await?;
            Ok(())
        })
    }

    fn photos_due<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        limit: i64,
    ) -> BoxFuture<'a, Result<Vec<DuePhoto>, AppError>> {
        Box::pin(async move {
            let photos = sqlx::query_as!(
                DuePhoto,
                r#"
                SELECT id AS capture_id, storage_key AS "storage_key!"
                FROM captures
                WHERE storage_key IS NOT NULL AND purge_after <= now()
                ORDER BY purge_after
                LIMIT $1
                "#,
                limit,
            )
            .fetch_all(conn)
            .await?;
            Ok(photos)
        })
    }

    fn forget_photo<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        photo: &'a DuePhoto,
    ) -> BoxFuture<'a, Result<bool, AppError>> {
        Box::pin(async move {
            let result = sqlx::query!(
                "UPDATE captures SET storage_key = NULL WHERE id = $1 AND storage_key = $2",
                photo.capture_id,
                photo.storage_key,
            )
            .execute(conn)
            .await?;
            Ok(result.rows_affected() == 1)
        })
    }
}

fn internal(error: serde_json::Error) -> AppError {
    AppError::new(ErrorCode::Internal).with_source(error)
}
