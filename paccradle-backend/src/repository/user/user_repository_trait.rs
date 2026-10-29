use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::user::{NewUser, User};
use crate::shared::future::BoxFuture;

/// Reads and writes `users`. Every method runs on the connection it is given, so a service can
/// put several in one transaction.
pub trait UserRepository: Send + Sync {
    /// The user with `id`.
    fn find<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        id: Uuid,
    ) -> BoxFuture<'a, Result<Option<User>, AppError>>;

    /// The account with `email`, whatever its letter case.
    fn find_by_email<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        email: &'a str,
    ) -> BoxFuture<'a, Result<Option<User>, AppError>>;

    /// The user's password hash, if they have set a password.
    fn password_hash<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        id: Uuid,
    ) -> BoxFuture<'a, Result<Option<String>, AppError>>;

    /// Adds a guest, or an account whose email is confirmed now.
    fn insert<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        user: &'a NewUser,
    ) -> BoxFuture<'a, Result<(), AppError>>;

    /// Puts a just-confirmed `email` on the guest `id`, making it an account. `false` when the
    /// user is gone or no longer a guest.
    fn confirm_guest_email<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        guest: (Uuid, &'a str),
    ) -> BoxFuture<'a, Result<bool, AppError>>;

    /// Replaces the account's password hash. `false` when the user is gone or a guest.
    fn set_password_hash<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        password: (Uuid, &'a str),
    ) -> BoxFuture<'a, Result<bool, AppError>>;

    /// Gives the user `to` the captures, products and designs of the user `from`.
    fn move_owned_rows<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        owners: (Uuid, Uuid),
    ) -> BoxFuture<'a, Result<(), AppError>>;

    /// Deletes the user and, by cascade, everything they own.
    fn delete<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        id: Uuid,
    ) -> BoxFuture<'a, Result<(), AppError>>;

    /// Holds a lock on `email` (any letter case) until the transaction ends, so two sign-ups
    /// for one address can't both make an account.
    fn lock_email<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        email: &'a str,
    ) -> BoxFuture<'a, Result<(), AppError>>;
}
