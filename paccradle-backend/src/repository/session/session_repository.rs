use std::time::Duration;

use sqlx::PgConnection;
use time::OffsetDateTime;
use uuid::Uuid;

use super::SessionRepository;
use crate::error::AppError;
use crate::models::TextEnum;
use crate::models::session::{NewSession, RefreshMatch, Rotation, Session, SignInMethod};
use crate::shared::api_surface::ApiSurface;
use crate::shared::future::BoxFuture;

/// [`SessionRepository`] on Postgres.
#[derive(Default)]
pub struct PgSessionRepository;

impl PgSessionRepository {
    /// The repository.
    pub fn new() -> Self {
        Self
    }
}

struct SessionRow {
    id: Uuid,
    user_id: Uuid,
    surface: String,
    method: String,
    authenticated_at: OffsetDateTime,
    expires_at: OffsetDateTime,
    revoked: bool,
}

impl TryFrom<SessionRow> for Session {
    type Error = AppError;

    fn try_from(row: SessionRow) -> Result<Self, AppError> {
        Ok(Self {
            id: row.id,
            user_id: row.user_id,
            surface: ApiSurface::from_text(&row.surface)?,
            method: SignInMethod::from_text(&row.method)?,
            authenticated_at: row.authenticated_at,
            expires_at: row.expires_at,
            revoked: row.revoked,
        })
    }
}

impl SessionRepository for PgSessionRepository {
    fn insert<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        session: &'a NewSession,
    ) -> BoxFuture<'a, Result<(), AppError>> {
        Box::pin(async move {
            sqlx::query!(
                r#"
                INSERT INTO sessions (id, user_id, surface, method, refresh_token_hash, expires_at)
                VALUES ($1, $2, $3, $4, $5, now() + make_interval(secs => $6))
                "#,
                session.id,
                session.user_id,
                session.surface.as_str(),
                session.method.as_str(),
                &session.refresh_token_hash[..],
                session.lifetime.as_secs_f64(),
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
    ) -> BoxFuture<'a, Result<Option<Session>, AppError>> {
        Box::pin(async move {
            let row = sqlx::query_as!(
                SessionRow,
                r#"
                SELECT id, user_id, surface, method, authenticated_at, expires_at,
                       revoked_at IS NOT NULL AS "revoked!"
                FROM sessions
                WHERE id = $1
                "#,
                id,
            )
            .fetch_optional(conn)
            .await?;
            row.map(Session::try_from).transpose()
        })
    }

    fn find_by_refresh_hash<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        (surface, hash): (ApiSurface, &'a [u8; 32]),
    ) -> BoxFuture<'a, Result<Option<RefreshMatch>, AppError>> {
        Box::pin(async move {
            let Some(row) = sqlx::query!(
                r#"
                SELECT id, user_id, surface, method, authenticated_at, expires_at,
                       revoked_at IS NOT NULL AS "revoked!",
                       refresh_token_hash = $2 AS "current!",
                       rotated_at
                FROM sessions
                WHERE surface = $1 AND (refresh_token_hash = $2 OR previous_refresh_token_hash = $2)
                "#,
                surface.as_str(),
                &hash[..],
            )
            .fetch_optional(conn)
            .await?
            else {
                return Ok(None);
            };
            let session = Session::try_from(SessionRow {
                id: row.id,
                user_id: row.user_id,
                surface: row.surface,
                method: row.method,
                authenticated_at: row.authenticated_at,
                expires_at: row.expires_at,
                revoked: row.revoked,
            })?;
            Ok(Some(match (row.current, row.rotated_at) {
                (false, Some(rotated_at)) => RefreshMatch::Previous {
                    session,
                    rotated_at,
                },
                _ => RefreshMatch::Current(session),
            }))
        })
    }

    fn rotate<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        rotation: &'a Rotation,
    ) -> BoxFuture<'a, Result<bool, AppError>> {
        Box::pin(async move {
            let result = sqlx::query!(
                r#"
                UPDATE sessions
                SET previous_refresh_token_hash = refresh_token_hash,
                    refresh_token_hash = $3,
                    rotated_at = now(),
                    expires_at = CASE
                        WHEN $4::float8 IS NULL THEN expires_at
                        ELSE now() + make_interval(secs => $4)
                    END
                WHERE id = $1 AND refresh_token_hash = $2 AND revoked_at IS NULL
                "#,
                rotation.session_id,
                &rotation.old_hash[..],
                &rotation.new_hash[..],
                rotation.lifetime.map(|lifetime| lifetime.as_secs_f64()),
            )
            .execute(conn)
            .await?;
            Ok(result.rows_affected() == 1)
        })
    }

    fn revoke<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        id: Uuid,
    ) -> BoxFuture<'a, Result<(), AppError>> {
        Box::pin(async move {
            sqlx::query!(
                "UPDATE sessions SET revoked_at = now() WHERE id = $1 AND revoked_at IS NULL",
                id,
            )
            .execute(conn)
            .await?;
            Ok(())
        })
    }

    fn revoke_others<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        (user_id, keep): (Uuid, Uuid),
    ) -> BoxFuture<'a, Result<u64, AppError>> {
        Box::pin(async move {
            let result = sqlx::query!(
                r#"
                UPDATE sessions SET revoked_at = now()
                WHERE user_id = $1 AND id <> $2 AND revoked_at IS NULL
                "#,
                user_id,
                keep,
            )
            .execute(conn)
            .await?;
            Ok(result.rows_affected())
        })
    }

    fn delete_ended<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        age: Duration,
    ) -> BoxFuture<'a, Result<u64, AppError>> {
        Box::pin(async move {
            let result = sqlx::query!(
                r#"
                DELETE FROM sessions WHERE expires_at < now() - make_interval(secs => $1)
                "#,
                age.as_secs_f64(),
            )
            .execute(conn)
            .await?;
            Ok(result.rows_affected())
        })
    }
}
