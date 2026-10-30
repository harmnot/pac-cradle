use serde_json::Value;
use sqlx::PgConnection;
use time::OffsetDateTime;
use uuid::Uuid;

use super::DesignRepository;
use crate::error::{AppError, ErrorCode};
use crate::models::TextEnum;
use crate::models::design::{DesignListQuery, DesignSummary, InsertStyle, NewDesign, SavedDesign};
use crate::shared::future::BoxFuture;

/// [`DesignRepository`] on Postgres.
#[derive(Default)]
pub struct PgDesignRepository;

impl PgDesignRepository {
    /// The repository.
    pub fn new() -> Self {
        Self
    }
}

struct DesignRow {
    id: Uuid,
    user_id: Uuid,
    name: String,
    spec: Value,
    version: i32,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

impl TryFrom<DesignRow> for SavedDesign {
    type Error = AppError;

    fn try_from(row: DesignRow) -> Result<Self, AppError> {
        Ok(Self {
            id: row.id,
            user_id: row.user_id,
            name: row.name,
            spec: serde_json::from_value(row.spec).map_err(internal)?,
            version: row.version,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

struct SummaryRow {
    id: Uuid,
    name: String,
    version: i32,
    insert_style: String,
    product_types: i32,
    product_count: i64,
    has_box: bool,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

impl TryFrom<SummaryRow> for DesignSummary {
    type Error = AppError;

    fn try_from(row: SummaryRow) -> Result<Self, AppError> {
        Ok(Self {
            id: row.id,
            name: row.name,
            version: row.version,
            insert_style: InsertStyle::from_text(&row.insert_style)?,
            product_types: row.product_types,
            product_count: row.product_count,
            has_box: row.has_box,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

impl DesignRepository for PgDesignRepository {
    fn insert<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        design: &'a NewDesign,
    ) -> BoxFuture<'a, Result<SavedDesign, AppError>> {
        Box::pin(async move {
            let spec = serde_json::to_value(&design.spec).map_err(internal)?;
            let row = sqlx::query_as!(
                DesignRow,
                r#"
                INSERT INTO designs (id, user_id, name, spec)
                VALUES ($1, $2, $3, $4)
                RETURNING id, user_id, name, spec, version, created_at, updated_at
                "#,
                design.id,
                design.user_id,
                design.name,
                spec,
            )
            .fetch_one(conn)
            .await?;
            SavedDesign::try_from(row)
        })
    }

    fn find<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        id: Uuid,
    ) -> BoxFuture<'a, Result<Option<SavedDesign>, AppError>> {
        Box::pin(async move {
            let row = sqlx::query_as!(
                DesignRow,
                r#"
                SELECT id, user_id, name, spec, version, created_at, updated_at
                FROM designs WHERE id = $1
                "#,
                id,
            )
            .fetch_optional(conn)
            .await?;
            row.map(SavedDesign::try_from).transpose()
        })
    }

    fn lock<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        id: Uuid,
    ) -> BoxFuture<'a, Result<Option<SavedDesign>, AppError>> {
        Box::pin(async move {
            let row = sqlx::query_as!(
                DesignRow,
                r#"
                SELECT id, user_id, name, spec, version, created_at, updated_at
                FROM designs WHERE id = $1
                FOR UPDATE
                "#,
                id,
            )
            .fetch_optional(conn)
            .await?;
            row.map(SavedDesign::try_from).transpose()
        })
    }

    fn update<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        design: &'a SavedDesign,
    ) -> BoxFuture<'a, Result<SavedDesign, AppError>> {
        Box::pin(async move {
            let spec = serde_json::to_value(&design.spec).map_err(internal)?;
            let row = sqlx::query_as!(
                DesignRow,
                r#"
                UPDATE designs SET name = $2, spec = $3, version = $4
                WHERE id = $1
                RETURNING id, user_id, name, spec, version, created_at, updated_at
                "#,
                design.id,
                design.name,
                spec,
                design.version,
            )
            .fetch_one(conn)
            .await?;
            SavedDesign::try_from(row)
        })
    }

    fn list<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        query: &'a DesignListQuery,
    ) -> BoxFuture<'a, Result<Vec<DesignSummary>, AppError>> {
        Box::pin(async move {
            // The sort column and direction are parameters (sqlx takes only fixed SQL): each
            // CASE sorts when its pair is asked for and is NULL, so ignored, otherwise.
            let rows = sqlx::query_as!(
                SummaryRow,
                r#"
                SELECT id, name, version,
                       spec -> 'insert' ->> 'style' AS "insert_style!",
                       jsonb_array_length(spec -> 'products') AS "product_types!",
                       (SELECT coalesce(sum((product ->> 'quantity')::bigint), 0)
                        FROM jsonb_array_elements(spec -> 'products') AS product)::bigint
                           AS "product_count!",
                       jsonb_typeof(spec -> 'outer_box') = 'object' AS "has_box!",
                       created_at, updated_at
                FROM designs
                WHERE user_id = $1
                  AND ($2::text IS NULL OR strpos(lower(name), lower($2)) > 0)
                  AND ($3::text IS NULL OR spec -> 'insert' ->> 'style' = $3)
                ORDER BY
                    CASE WHEN $4 = 'name' AND $5 = 'asc' THEN lower(name) END ASC,
                    CASE WHEN $4 = 'name' AND $5 = 'desc' THEN lower(name) END DESC,
                    CASE WHEN $4 = 'created_at' AND $5 = 'asc' THEN created_at END ASC,
                    CASE WHEN $4 = 'created_at' AND $5 = 'desc' THEN created_at END DESC,
                    CASE WHEN $4 = 'updated_at' AND $5 = 'asc' THEN updated_at END ASC,
                    CASE WHEN $4 = 'updated_at' AND $5 = 'desc' THEN updated_at END DESC,
                    id
                LIMIT $6 OFFSET $7
                "#,
                query.user_id,
                query.search,
                query.style.map(TextEnum::as_str),
                query.sort.as_str(),
                query.order.as_str(),
                query.page.limit(),
                query.page.offset(),
            )
            .fetch_all(conn)
            .await?;
            rows.into_iter().map(DesignSummary::try_from).collect()
        })
    }

    fn count<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        query: &'a DesignListQuery,
    ) -> BoxFuture<'a, Result<u64, AppError>> {
        Box::pin(async move {
            let count = sqlx::query_scalar!(
                r#"
                SELECT count(*) AS "count!"
                FROM designs
                WHERE user_id = $1
                  AND ($2::text IS NULL OR strpos(lower(name), lower($2)) > 0)
                  AND ($3::text IS NULL OR spec -> 'insert' ->> 'style' = $3)
                "#,
                query.user_id,
                query.search,
                query.style.map(TextEnum::as_str),
            )
            .fetch_one(conn)
            .await?;
            Ok(u64::try_from(count).unwrap_or_default())
        })
    }
}

fn internal(error: serde_json::Error) -> AppError {
    AppError::new(ErrorCode::Internal).with_source(error)
}
