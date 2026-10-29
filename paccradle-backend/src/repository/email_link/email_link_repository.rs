use std::time::Duration;

use sqlx::PgConnection;
use time::OffsetDateTime;
use uuid::Uuid;

use super::EmailLinkRepository;
use crate::error::AppError;
use crate::models::TextEnum;
use crate::models::email_link::{EmailLink, EmailLinkPurpose, NewEmailLink};
use crate::shared::future::BoxFuture;

/// [`EmailLinkRepository`] on Postgres.
#[derive(Default)]
pub struct PgEmailLinkRepository;

impl PgEmailLinkRepository {
    /// The repository.
    pub fn new() -> Self {
        Self
    }
}

struct EmailLinkRow {
    id: Uuid,
    email: String,
    purpose: String,
    locale: String,
    created_at: OffsetDateTime,
    used: bool,
}

impl TryFrom<EmailLinkRow> for EmailLink {
    type Error = AppError;

    fn try_from(row: EmailLinkRow) -> Result<Self, AppError> {
        Ok(Self {
            id: row.id,
            email: row.email,
            purpose: EmailLinkPurpose::from_text(&row.purpose)?,
            locale: row.locale,
            created_at: row.created_at,
            used: row.used,
        })
    }
}

impl EmailLinkRepository for PgEmailLinkRepository {
    fn insert<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        link: &'a NewEmailLink,
    ) -> BoxFuture<'a, Result<(), AppError>> {
        Box::pin(async move {
            sqlx::query!(
                "INSERT INTO email_links (id, email, purpose, locale) VALUES ($1, $2, $3, $4)",
                link.id,
                link.email,
                link.purpose.as_str(),
                link.locale,
            )
            .execute(conn)
            .await?;
            Ok(())
        })
    }

    fn count_recent<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        (email, within): (&'a str, Duration),
    ) -> BoxFuture<'a, Result<i64, AppError>> {
        Box::pin(async move {
            let count = sqlx::query_scalar!(
                r#"
                SELECT count(*) AS "count!" FROM email_links
                WHERE lower(email) = lower($1) AND created_at > now() - make_interval(secs => $2)
                "#,
                email,
                within.as_secs_f64(),
            )
            .fetch_one(conn)
            .await?;
            Ok(count)
        })
    }

    fn find<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        id: Uuid,
    ) -> BoxFuture<'a, Result<Option<EmailLink>, AppError>> {
        Box::pin(async move {
            let row = sqlx::query_as!(
                EmailLinkRow,
                r#"
                SELECT id, email, purpose, locale, created_at, used_at IS NOT NULL AS "used!"
                FROM email_links
                WHERE id = $1
                "#,
                id,
            )
            .fetch_optional(conn)
            .await?;
            row.map(EmailLink::try_from).transpose()
        })
    }

    fn set_token<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        (id, hash, lifetime): (Uuid, &'a [u8; 32], Duration),
    ) -> BoxFuture<'a, Result<bool, AppError>> {
        Box::pin(async move {
            let result = sqlx::query!(
                r#"
                UPDATE email_links
                SET token_hash = $2, expires_at = now() + make_interval(secs => $3)
                WHERE id = $1 AND used_at IS NULL
                "#,
                id,
                &hash[..],
                lifetime.as_secs_f64(),
            )
            .execute(conn)
            .await?;
            Ok(result.rows_affected() == 1)
        })
    }

    fn consume<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        hash: &'a [u8; 32],
    ) -> BoxFuture<'a, Result<Option<EmailLink>, AppError>> {
        Box::pin(async move {
            let row = sqlx::query_as!(
                EmailLinkRow,
                r#"
                UPDATE email_links SET used_at = now()
                WHERE token_hash = $1 AND used_at IS NULL AND expires_at > now()
                RETURNING id, email, purpose, locale, created_at, true AS "used!"
                "#,
                &hash[..],
            )
            .fetch_optional(conn)
            .await?;
            row.map(EmailLink::try_from).transpose()
        })
    }

    fn retire_unused<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        email: &'a str,
    ) -> BoxFuture<'a, Result<u64, AppError>> {
        Box::pin(async move {
            let result = sqlx::query!(
                r#"
                UPDATE email_links SET used_at = now()
                WHERE lower(email) = lower($1) AND used_at IS NULL
                "#,
                email,
            )
            .execute(conn)
            .await?;
            Ok(result.rows_affected())
        })
    }

    fn delete_older_than<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        age: Duration,
    ) -> BoxFuture<'a, Result<u64, AppError>> {
        Box::pin(async move {
            let result = sqlx::query!(
                "DELETE FROM email_links WHERE created_at < now() - make_interval(secs => $1)",
                age.as_secs_f64(),
            )
            .execute(conn)
            .await?;
            Ok(result.rows_affected())
        })
    }
}
