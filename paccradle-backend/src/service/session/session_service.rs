use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sqlx::PgConnection;
use time::OffsetDateTime;
use uuid::Uuid;

use super::{SessionService, StartSession};
use crate::error::{AppError, ErrorCode};
use crate::models::TextEnum;
use crate::models::session::{
    AuthUser, NewSession, NextStep, RefreshMatch, Rotation, Session, SignInMethod, SignedIn,
};
use crate::models::user::{AccountRole, User};
use crate::repository::database::Database;
use crate::repository::session::SessionRepository;
use crate::repository::user::UserRepository;
use crate::shared::api_surface::ApiSurface;
use crate::shared::crypto::{NewToken, token_hash};
use crate::shared::future::BoxFuture;
use crate::shared::id::new_id;
use crate::shared::jwt::{self, JwtKey};

/// How long a web app access token lasts.
pub const CLIENT_ACCESS_TTL: Duration = Duration::from_secs(15 * 60);
/// How long a back-office access token lasts: admin rights can be taken away within minutes.
pub const ADMIN_ACCESS_TTL: Duration = Duration::from_secs(5 * 60);
/// A web app session lasts this long after its last refresh.
pub const CLIENT_SESSION_TTL: Duration = Duration::from_secs(30 * 24 * 60 * 60);
/// A back-office session lasts this long after sign-in, however often it refreshes.
pub const ADMIN_SESSION_TTL: Duration = Duration::from_secs(12 * 60 * 60);
/// A refresh token swapped less than this long ago is still answered (two tabs racing),
/// without a new refresh token: the browser already has the newer one.
pub const REFRESH_RACE_GRACE: Duration = Duration::from_secs(30);
/// Sessions that expired this long ago are deleted at the next sign-in.
const ENDED_SESSION_RETENTION: Duration = Duration::from_secs(30 * 24 * 60 * 60);

/// How sessions behave on a surface.
struct Policy {
    access_ttl: Duration,
    session_ttl: Duration,
    /// Whether a refresh pushes the session's end back.
    sliding: bool,
}

const fn policy(surface: ApiSurface) -> Policy {
    match surface {
        ApiSurface::Client => Policy {
            access_ttl: CLIENT_ACCESS_TTL,
            session_ttl: CLIENT_SESSION_TTL,
            sliding: true,
        },
        ApiSurface::Admin => Policy {
            access_ttl: ADMIN_ACCESS_TTL,
            session_ttl: ADMIN_SESSION_TTL,
            sliding: false,
        },
    }
}

/// An access token's claims.
#[derive(Serialize, Deserialize)]
struct AccessClaims {
    /// The user.
    sub: Uuid,
    /// The session.
    sid: Uuid,
    /// The surface it is valid on: `client` or `admin`.
    aud: String,
    /// `guest`, `user` or `admin`.
    role: String,
    /// Issued at, Unix seconds.
    iat: u64,
    /// Expires at, Unix seconds.
    exp: u64,
}

/// [`SessionService`] on Postgres, signing access tokens with `AUTH_JWT_SECRET`.
pub struct DefaultSessionService {
    db: Arc<dyn Database>,
    sessions: Arc<dyn SessionRepository>,
    users: Arc<dyn UserRepository>,
    key: JwtKey,
}

impl DefaultSessionService {
    /// A service on these repositories, signing with `secret`.
    pub fn new(
        db: Arc<dyn Database>,
        (sessions, users): (Arc<dyn SessionRepository>, Arc<dyn UserRepository>),
        secret: &[u8],
    ) -> Self {
        Self {
            db,
            sessions,
            users,
            key: JwtKey::new(secret),
        }
    }

    fn signed_in(
        &self,
        (user, session_id, surface): (User, Uuid, ApiSurface),
        refresh: Option<(String, Duration)>,
        next_steps: Vec<NextStep>,
    ) -> Result<SignedIn, AppError> {
        let access_ttl = policy(surface).access_ttl;
        let iat = unix_now();
        let claims = AccessClaims {
            sub: user.id,
            sid: session_id,
            aud: surface.name().to_owned(),
            role: user.account_role().as_str().to_owned(),
            iat,
            exp: iat + access_ttl.as_secs(),
        };
        let access_token = jwt::encode(&self.key, &claims)
            .map_err(|error| AppError::new(ErrorCode::Internal).with_source(error))?;
        Ok(SignedIn {
            user,
            session_id,
            surface,
            access_token,
            access_expires_in: access_ttl,
            refresh,
            next_steps,
        })
    }

    /// Ends the session `replacing` belongs to; a guest's goes with its guest, whose rows move
    /// to `user_id` first.
    async fn replace(
        &self,
        conn: &mut PgConnection,
        start: &StartSession<'_>,
    ) -> Result<(), AppError> {
        let Some(token) = start.replacing else {
            return Ok(());
        };
        let hash = token_hash(token);
        let found = self
            .sessions
            .find_by_refresh_hash(conn, (start.surface, &hash))
            .await?;
        let Some(RefreshMatch::Current(old)) = found else {
            return Ok(());
        };
        if old.revoked {
            return Ok(());
        }
        let old_user = self.users.find(conn, old.user_id).await?;
        let merge_guest = old.user_id != start.user_id
            && start.surface == ApiSurface::Client
            && old_user.as_ref().is_some_and(User::is_guest);
        if merge_guest {
            self.users
                .move_owned_rows(conn, (old.user_id, start.user_id))
                .await?;
            // Its sessions go with it.
            self.users.delete(conn, old.user_id).await?;
            tracing::info!(guest = %old.user_id, user = %start.user_id, "guest merged at sign-in");
            return Ok(());
        }
        self.sessions.revoke(conn, old.id).await
    }

    async fn refresh_in(
        &self,
        conn: &mut PgConnection,
        (surface, token): (ApiSurface, &str),
    ) -> Result<Refreshed, AppError> {
        let hash = token_hash(token);
        let found = self
            .sessions
            .find_by_refresh_hash(conn, (surface, &hash))
            .await?;
        let now = OffsetDateTime::now_utc();
        let (session, refresh) = match found {
            None => return Ok(Refreshed::Ended),
            Some(RefreshMatch::Current(session)) => {
                if !is_live(&session, now) {
                    return Ok(Refreshed::Ended);
                }
                let new = NewToken::generate()?;
                let policy = policy(surface);
                let rotation = Rotation {
                    session_id: session.id,
                    old_hash: hash,
                    new_hash: new.hash,
                    lifetime: policy.sliding.then_some(policy.session_ttl),
                };
                // Another refresh may have swapped it a moment ago: then this one is the
                // losing side of a race, answered without a new refresh token.
                let rotated = self.sessions.rotate(conn, &rotation).await?;
                let lasts = match policy.sliding {
                    true => policy.session_ttl,
                    false => remaining(&session, now),
                };
                (session, rotated.then_some((new.token, lasts)))
            }
            Some(RefreshMatch::Previous {
                session,
                rotated_at,
            }) => {
                if !is_live(&session, now) {
                    return Ok(Refreshed::Ended);
                }
                if now - rotated_at > REFRESH_RACE_GRACE {
                    // An old token came back: someone else may hold the session. End it.
                    self.sessions.revoke(conn, session.id).await?;
                    tracing::warn!(session = %session.id, "refresh token reused; session revoked");
                    return Ok(Refreshed::Revoked);
                }
                (session, None)
            }
        };
        let user = self.users.find(conn, session.user_id).await?;
        let Some(user) = user else {
            return Ok(Refreshed::Ended);
        };
        if surface == ApiSurface::Admin && user.account_role() != AccountRole::Admin {
            self.sessions.revoke(conn, session.id).await?;
            return Ok(Refreshed::Revoked);
        }
        Ok(Refreshed::Live {
            user,
            session_id: session.id,
            refresh,
        })
    }
}

/// How a refresh went, before committing.
enum Refreshed {
    /// The session continues.
    Live {
        user: User,
        session_id: Uuid,
        refresh: Option<(String, Duration)>,
    },
    /// Unknown token, or the session expired or was revoked earlier.
    Ended,
    /// The session was revoked just now (token reuse, admin role gone): commit that first.
    Revoked,
}

impl SessionService for DefaultSessionService {
    fn start<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        start: StartSession<'a>,
    ) -> BoxFuture<'a, Result<SignedIn, AppError>> {
        Box::pin(async move {
            self.replace(conn, &start).await?;
            let user = self
                .users
                .find(conn, start.user_id)
                .await?
                .ok_or_else(|| AppError::new(ErrorCode::AuthRequired))?;
            if start.surface == ApiSurface::Admin && user.account_role() != AccountRole::Admin {
                return Err(AppError::new(ErrorCode::AuthInvalidCredentials));
            }
            let policy = policy(start.surface);
            let token = NewToken::generate()?;
            let session_id = new_id();
            self.sessions
                .insert(
                    conn,
                    &NewSession {
                        id: session_id,
                        user_id: user.id,
                        surface: start.surface,
                        method: start.method,
                        refresh_token_hash: token.hash,
                        lifetime: policy.session_ttl,
                    },
                )
                .await?;
            self.sessions
                .delete_ended(conn, ENDED_SESSION_RETENTION)
                .await?;
            let next_steps = next_steps(&user, (start.surface, start.method));
            self.signed_in(
                (user, session_id, start.surface),
                Some((token.token, policy.session_ttl)),
                next_steps,
            )
        })
    }

    fn refresh<'a>(
        &'a self,
        token: (ApiSurface, &'a str),
    ) -> BoxFuture<'a, Result<SignedIn, AppError>> {
        Box::pin(async move {
            let mut tx = self.db.begin().await?;
            let refreshed = self.refresh_in(&mut tx, token).await?;
            tx.commit().await?;
            match refreshed {
                Refreshed::Live {
                    user,
                    session_id,
                    refresh,
                } => self.signed_in((user, session_id, token.0), refresh, Vec::new()),
                Refreshed::Ended | Refreshed::Revoked => {
                    Err(AppError::new(ErrorCode::AuthSessionExpired))
                }
            }
        })
    }

    fn sign_out<'a>(
        &'a self,
        (surface, token): (ApiSurface, &'a str),
    ) -> BoxFuture<'a, Result<(), AppError>> {
        Box::pin(async move {
            let mut conn = self.db.connection().await?;
            let hash = token_hash(token);
            let found = self
                .sessions
                .find_by_refresh_hash(&mut conn, (surface, &hash))
                .await?;
            let session = match found {
                Some(RefreshMatch::Current(session) | RefreshMatch::Previous { session, .. }) => {
                    session
                }
                None => return Ok(()),
            };
            self.sessions.revoke(&mut conn, session.id).await
        })
    }

    fn authenticate(&self, (surface, token): (ApiSurface, &str)) -> Result<AuthUser, AppError> {
        let refused = |reason: &'static str| {
            tracing::debug!(reason, "access token refused");
            AppError::new(ErrorCode::AuthRequired)
        };
        let claims: AccessClaims =
            jwt::decode(&self.key, token).map_err(|_| refused("bad token"))?;
        if claims.aud != surface.name() {
            return Err(refused("another surface"));
        }
        if claims.exp <= unix_now() {
            return Err(refused("expired"));
        }
        let role = AccountRole::from_text(&claims.role).map_err(|_| refused("unknown role"))?;
        Ok(AuthUser {
            user_id: claims.sub,
            session_id: claims.sid,
            surface,
            role,
        })
    }

    fn current_guest<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        (surface, token): (ApiSurface, &'a str),
    ) -> BoxFuture<'a, Result<Option<Uuid>, AppError>> {
        Box::pin(async move {
            let hash = token_hash(token);
            let found = self
                .sessions
                .find_by_refresh_hash(conn, (surface, &hash))
                .await?;
            let Some(RefreshMatch::Current(session)) = found else {
                return Ok(None);
            };
            if !is_live(&session, OffsetDateTime::now_utc()) {
                return Ok(None);
            }
            let user = self.users.find(conn, session.user_id).await?;
            Ok(user.filter(User::is_guest).map(|user| user.id))
        })
    }

    fn find(&self, id: Uuid) -> BoxFuture<'_, Result<Option<Session>, AppError>> {
        Box::pin(async move {
            let mut conn = self.db.connection().await?;
            self.sessions.find(&mut conn, id).await
        })
    }

    fn end_others<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        keep: (Uuid, Uuid),
    ) -> BoxFuture<'a, Result<u64, AppError>> {
        self.sessions.revoke_others(conn, keep)
    }
}

/// What the web app should offer after a sign-in: a password if the account has none, and a
/// passkey on this device unless the sign-in used one. Nothing for guests or the back office.
fn next_steps(user: &User, (surface, method): (ApiSurface, SignInMethod)) -> Vec<NextStep> {
    if user.is_guest() || surface == ApiSurface::Admin {
        return Vec::new();
    }
    let mut steps = Vec::new();
    if !user.has_password {
        steps.push(NextStep::SetPassword);
    }
    if method != SignInMethod::Passkey {
        steps.push(NextStep::AddPasskey);
    }
    steps
}

fn is_live(session: &Session, now: OffsetDateTime) -> bool {
    !session.revoked && session.expires_at > now
}

/// Time left until the session ends.
fn remaining(session: &Session, now: OffsetDateTime) -> Duration {
    Duration::try_from(session.expires_at - now).unwrap_or_default()
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}
