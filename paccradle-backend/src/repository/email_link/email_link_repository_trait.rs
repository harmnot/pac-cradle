use std::time::Duration;

use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::email_link::{EmailLink, NewEmailLink};
use crate::shared::future::BoxFuture;

/// Reads and writes `email_links`, on the connection it is given.
pub trait EmailLinkRepository: Send + Sync {
    /// Adds a link, not yet sent.
    fn insert<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        link: &'a NewEmailLink,
    ) -> BoxFuture<'a, Result<(), AppError>>;

    /// How many links were asked for `email` (any letter case) in the last `within`.
    fn count_recent<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        recent: (&'a str, Duration),
    ) -> BoxFuture<'a, Result<i64, AppError>>;

    /// The link with `id`.
    fn find<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        id: Uuid,
    ) -> BoxFuture<'a, Result<Option<EmailLink>, AppError>>;

    /// Gives the unused link its token's hash, valid for `lifetime` from now (replacing an
    /// earlier one). `false` when the link is gone or used.
    fn set_token<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        token: (Uuid, &'a [u8; 32], Duration),
    ) -> BoxFuture<'a, Result<bool, AppError>>;

    /// Marks the unused, unexpired link whose token hashes to `hash` used, and returns it.
    fn consume<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        hash: &'a [u8; 32],
    ) -> BoxFuture<'a, Result<Option<EmailLink>, AppError>>;

    /// Marks every unused link for `email` (any letter case) used, so none works any more.
    fn retire_unused<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        email: &'a str,
    ) -> BoxFuture<'a, Result<u64, AppError>>;

    /// Deletes links asked for more than `age` ago. Returns how many.
    fn delete_older_than<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        age: Duration,
    ) -> BoxFuture<'a, Result<u64, AppError>>;
}
