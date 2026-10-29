use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::session::{AuthUser, Session, SignInMethod, SignedIn};
use crate::shared::api_surface::ApiSurface;
use crate::shared::future::BoxFuture;

/// A sign-in to record.
#[derive(Debug, Clone, Copy)]
pub struct StartSession<'a> {
    /// Who signed in.
    pub user_id: Uuid,
    /// Where.
    pub surface: ApiSurface,
    /// How.
    pub method: SignInMethod,
    /// The browser's refresh token (cookie), if it had one. Its session ends; when it was a
    /// guest's, the guest's captures, products and designs move to `user_id` and the guest
    /// goes.
    pub replacing: Option<&'a str>,
}

/// Sessions and the tokens that prove them.
pub trait SessionService: Send + Sync {
    /// Starts a session for a user who just proved who they are, inside the caller's
    /// transaction, and returns its tokens.
    fn start<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        start: StartSession<'a>,
    ) -> BoxFuture<'a, Result<SignedIn, AppError>>;

    /// Swaps a refresh token for a new access token and a new refresh token. A token that was
    /// already swapped ends the session (it was stolen, or the browser misbehaves), except
    /// within a few seconds of the swap, when two tabs raced and both are answered.
    /// `AUTH_SESSION_EXPIRED` when the session can't continue.
    fn refresh<'a>(
        &'a self,
        token: (ApiSurface, &'a str),
    ) -> BoxFuture<'a, Result<SignedIn, AppError>>;

    /// Ends the session the refresh token belongs to, if any.
    fn sign_out<'a>(&'a self, token: (ApiSurface, &'a str)) -> BoxFuture<'a, Result<(), AppError>>;

    /// Who an access token was issued to, if it is valid for `surface` and unexpired.
    /// `AUTH_REQUIRED` otherwise.
    fn authenticate(&self, token: (ApiSurface, &str)) -> Result<AuthUser, AppError>;

    /// The guest a live refresh token on `surface` belongs to, if it belongs to a guest.
    fn current_guest<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        token: (ApiSurface, &'a str),
    ) -> BoxFuture<'a, Result<Option<Uuid>, AppError>>;

    /// The session with `id`.
    fn find(&self, id: Uuid) -> BoxFuture<'_, Result<Option<Session>, AppError>>;

    /// Ends every other session of the user, inside the caller's transaction (after a
    /// password change).
    fn end_others<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        keep: (Uuid, Uuid),
    ) -> BoxFuture<'a, Result<u64, AppError>>;
}
