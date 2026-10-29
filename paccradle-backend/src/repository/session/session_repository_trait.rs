use std::time::Duration;

use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::session::{NewSession, RefreshMatch, Rotation, Session};
use crate::shared::api_surface::ApiSurface;
use crate::shared::future::BoxFuture;

/// Reads and writes `sessions`, on the connection it is given.
pub trait SessionRepository: Send + Sync {
    /// Adds a session.
    fn insert<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        session: &'a NewSession,
    ) -> BoxFuture<'a, Result<(), AppError>>;

    /// The session with `id`.
    fn find<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        id: Uuid,
    ) -> BoxFuture<'a, Result<Option<Session>, AppError>>;

    /// The session on `surface` whose current or previous refresh token hashes to `hash`.
    fn find_by_refresh_hash<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        token: (ApiSurface, &'a [u8; 32]),
    ) -> BoxFuture<'a, Result<Option<RefreshMatch>, AppError>>;

    /// Replaces the session's refresh token, keeping the old one as its previous. `false` when
    /// the old token is no longer current (another refresh won) or the session was revoked.
    fn rotate<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        rotation: &'a Rotation,
    ) -> BoxFuture<'a, Result<bool, AppError>>;

    /// Ends the session.
    fn revoke<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        id: Uuid,
    ) -> BoxFuture<'a, Result<(), AppError>>;

    /// Ends every session of `user_id` except `keep`. Returns how many.
    fn revoke_others<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        owner: (Uuid, Uuid),
    ) -> BoxFuture<'a, Result<u64, AppError>>;

    /// Deletes sessions that expired more than `age` ago (revoked ones included). Returns how
    /// many.
    fn delete_ended<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        age: Duration,
    ) -> BoxFuture<'a, Result<u64, AppError>>;
}
