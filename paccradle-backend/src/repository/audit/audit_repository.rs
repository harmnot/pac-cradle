use sqlx::PgConnection;
use uuid::Uuid;

use super::AuditRepository;
use crate::error::{AppError, ErrorCode};
use crate::models::audit::AdminWrite;
use crate::shared::future::BoxFuture;

/// [`AuditRepository`] on Postgres.
#[derive(Default)]
pub struct PgAuditRepository;

impl PgAuditRepository {
    /// The repository.
    pub fn new() -> Self {
        Self
    }
}

impl AuditRepository for PgAuditRepository {
    fn insert<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        write: &'a AdminWrite,
    ) -> BoxFuture<'a, Result<(), AppError>> {
        Box::pin(async move {
            // The email is copied so the entry still names the admin after their account goes.
            let result = sqlx::query!(
                r#"
                INSERT INTO admin_audit_log
                    (id, admin_id, admin_email, session_id, request_id, method, route, path)
                SELECT $1, users.id, users.email, $3, $4, $5, $6, $7
                FROM users
                WHERE users.id = $2 AND users.email IS NOT NULL
                "#,
                write.id,
                write.admin_id,
                write.session_id,
                write.request_id,
                write.method,
                write.route,
                write.path,
            )
            .execute(conn)
            .await?;
            if result.rows_affected() != 1 {
                // The admin's account went between sign-in and now: refuse the write.
                return Err(AppError::new(ErrorCode::AuthRequired));
            }
            Ok(())
        })
    }

    fn finish<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        (id, status): (Uuid, u16),
    ) -> BoxFuture<'a, Result<(), AppError>> {
        Box::pin(async move {
            let status = i16::try_from(status)
                .map_err(|error| AppError::new(ErrorCode::Internal).with_source(error))?;
            sqlx::query!(
                r#"
                UPDATE admin_audit_log SET status = $2, finished_at = now()
                WHERE id = $1 AND status IS NULL
                "#,
                id,
                status,
            )
            .execute(conn)
            .await?;
            Ok(())
        })
    }
}
