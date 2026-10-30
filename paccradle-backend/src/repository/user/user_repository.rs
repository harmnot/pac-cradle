use sqlx::PgConnection;
use time::OffsetDateTime;
use uuid::Uuid;

use super::UserRepository;
use crate::error::AppError;
use crate::models::TextEnum;
use crate::models::paper::Paper;
use crate::models::user::{NewUser, Role, User};
use crate::shared::future::BoxFuture;

/// [`UserRepository`] on Postgres.
#[derive(Default)]
pub struct PgUserRepository;

impl PgUserRepository {
    /// The repository.
    pub fn new() -> Self {
        Self
    }
}

struct UserRow {
    id: Uuid,
    email: Option<String>,
    role: String,
    locale: String,
    paper_default: String,
    has_password: bool,
    passkey_count: i64,
    created_at: OffsetDateTime,
}

impl TryFrom<UserRow> for User {
    type Error = AppError;

    fn try_from(row: UserRow) -> Result<Self, AppError> {
        Ok(Self {
            id: row.id,
            email: row.email,
            role: Role::from_text(&row.role)?,
            locale: row.locale,
            paper_default: Paper::from_text(&row.paper_default)?,
            has_password: row.has_password,
            passkey_count: row.passkey_count,
            created_at: row.created_at,
        })
    }
}

impl UserRepository for PgUserRepository {
    fn find<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        id: Uuid,
    ) -> BoxFuture<'a, Result<Option<User>, AppError>> {
        Box::pin(async move {
            let row = sqlx::query_as!(
                UserRow,
                r#"
                SELECT id, email, role, locale, paper_default,
                       password_hash IS NOT NULL AS "has_password!",
                       (SELECT count(*) FROM passkeys WHERE passkeys.user_id = users.id)
                           AS "passkey_count!",
                       created_at
                FROM users
                WHERE id = $1
                "#,
                id,
            )
            .fetch_optional(conn)
            .await?;
            row.map(User::try_from).transpose()
        })
    }

    fn find_by_email<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        email: &'a str,
    ) -> BoxFuture<'a, Result<Option<User>, AppError>> {
        Box::pin(async move {
            let row = sqlx::query_as!(
                UserRow,
                r#"
                SELECT id, email, role, locale, paper_default,
                       password_hash IS NOT NULL AS "has_password!",
                       (SELECT count(*) FROM passkeys WHERE passkeys.user_id = users.id)
                           AS "passkey_count!",
                       created_at
                FROM users
                WHERE lower(email) = lower($1)
                "#,
                email,
            )
            .fetch_optional(conn)
            .await?;
            row.map(User::try_from).transpose()
        })
    }

    fn password_hash<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        id: Uuid,
    ) -> BoxFuture<'a, Result<Option<String>, AppError>> {
        Box::pin(async move {
            let hash = sqlx::query_scalar!("SELECT password_hash FROM users WHERE id = $1", id)
                .fetch_optional(conn)
                .await?;
            Ok(hash.flatten())
        })
    }

    fn insert<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        user: &'a NewUser,
    ) -> BoxFuture<'a, Result<(), AppError>> {
        Box::pin(async move {
            sqlx::query!(
                r#"
                INSERT INTO users (id, email, email_confirmed_at, locale)
                VALUES ($1, $2, CASE WHEN $2::text IS NULL THEN NULL ELSE now() END, $3)
                "#,
                user.id,
                user.email,
                user.locale,
            )
            .execute(conn)
            .await?;
            Ok(())
        })
    }

    fn confirm_guest_email<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        (id, email): (Uuid, &'a str),
    ) -> BoxFuture<'a, Result<bool, AppError>> {
        Box::pin(async move {
            let result = sqlx::query!(
                r#"
                UPDATE users SET email = $2, email_confirmed_at = now()
                WHERE id = $1 AND email IS NULL
                "#,
                id,
                email,
            )
            .execute(conn)
            .await?;
            Ok(result.rows_affected() == 1)
        })
    }

    fn set_password_hash<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        (id, hash): (Uuid, &'a str),
    ) -> BoxFuture<'a, Result<bool, AppError>> {
        Box::pin(async move {
            let result = sqlx::query!(
                "UPDATE users SET password_hash = $2 WHERE id = $1 AND email IS NOT NULL",
                id,
                hash,
            )
            .execute(conn)
            .await?;
            Ok(result.rows_affected() == 1)
        })
    }

    fn move_owned_rows<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        (from, to): (Uuid, Uuid),
    ) -> BoxFuture<'a, Result<(), AppError>> {
        Box::pin(async move {
            sqlx::query!(
                "UPDATE captures SET user_id = $2 WHERE user_id = $1",
                from,
                to
            )
            .execute(&mut *conn)
            .await?;
            sqlx::query!(
                "UPDATE products SET user_id = $2 WHERE user_id = $1",
                from,
                to
            )
            .execute(&mut *conn)
            .await?;
            sqlx::query!(
                "UPDATE designs SET user_id = $2 WHERE user_id = $1",
                from,
                to
            )
            .execute(&mut *conn)
            .await?;
            Ok(())
        })
    }

    fn delete<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        id: Uuid,
    ) -> BoxFuture<'a, Result<(), AppError>> {
        Box::pin(async move {
            sqlx::query!("DELETE FROM users WHERE id = $1", id)
                .execute(conn)
                .await?;
            Ok(())
        })
    }

    fn lock_email<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        email: &'a str,
    ) -> BoxFuture<'a, Result<(), AppError>> {
        Box::pin(async move {
            // count(*) makes Postgres take the lock without returning its void result.
            sqlx::query_scalar!(
                r#"
                SELECT count(*) AS "locked!"
                FROM (SELECT pg_advisory_xact_lock(hashtextextended(lower($1), 0))) AS lock
                "#,
                email,
            )
            .fetch_one(conn)
            .await?;
            Ok(())
        })
    }
}
