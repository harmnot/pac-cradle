//! Signed-in browsers (the `sessions` table) and who a request comes from.

use std::time::Duration;

use serde::Serialize;
use time::OffsetDateTime;
use uuid::Uuid;

use super::TextEnum;
use super::user::{AccountRole, User};
use crate::shared::api_surface::ApiSurface;

impl TextEnum for ApiSurface {
    const ALL: &'static [Self] = &ApiSurface::ALL;

    fn as_str(self) -> &'static str {
        self.name()
    }
}

/// `sessions.method`: how the user proved who they are.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignInMethod {
    /// Nothing: a guest session.
    Guest,
    /// A link sent to their email.
    EmailLink,
    /// Email and password.
    Password,
    /// A passkey.
    Passkey,
}

impl TextEnum for SignInMethod {
    const ALL: &'static [Self] = &[Self::Guest, Self::EmailLink, Self::Password, Self::Passkey];

    fn as_str(self) -> &'static str {
        match self {
            Self::Guest => "guest",
            Self::EmailLink => "email_link",
            Self::Password => "password",
            Self::Passkey => "passkey",
        }
    }
}

/// A session as stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    /// The session's id (the `sid` of its access tokens).
    pub id: Uuid,
    /// Whose it is.
    pub user_id: Uuid,
    /// Which surface it signs in to.
    pub surface: ApiSurface,
    /// How it began.
    pub method: SignInMethod,
    /// When the user last proved who they are in it.
    pub authenticated_at: OffsetDateTime,
    /// When it ends unless refreshed (client) or for good (admin).
    pub expires_at: OffsetDateTime,
    /// Whether it was signed out or revoked.
    pub revoked: bool,
}

/// A session to add.
#[derive(Debug, Clone)]
pub struct NewSession {
    /// The id it will have.
    pub id: Uuid,
    /// Whose it is.
    pub user_id: Uuid,
    /// Which surface it signs in to.
    pub surface: ApiSurface,
    /// How it began.
    pub method: SignInMethod,
    /// The hash of its first refresh token.
    pub refresh_token_hash: [u8; 32],
    /// How long until it expires.
    pub lifetime: Duration,
}

/// The session a refresh token belongs to, and whether it is the current token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefreshMatch {
    /// The session's current token.
    Current(Session),
    /// The token it had before its last rotation.
    Previous {
        /// The session.
        session: Session,
        /// When the token was replaced.
        rotated_at: OffsetDateTime,
    },
}

/// A refresh token's replacement.
#[derive(Debug, Clone)]
pub struct Rotation {
    /// The session.
    pub session_id: Uuid,
    /// The token being replaced: the rotation only happens while it is still current.
    pub old_hash: [u8; 32],
    /// The new token's hash.
    pub new_hash: [u8; 32],
    /// A new lifetime from now, or `None` to keep the session's end (admin sessions).
    pub lifetime: Option<Duration>,
}

/// Who a request comes from, read from its access token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthUser {
    /// The user.
    pub user_id: Uuid,
    /// Their session.
    pub session_id: Uuid,
    /// The surface the token was issued for.
    pub surface: ApiSurface,
    /// What they act as, as of the token's issue.
    pub role: AccountRole,
}

/// What the web app should offer right after a sign-in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NextStep {
    /// The account has no password yet: ask for one (`PUT /me/password`).
    SetPassword,
    /// The sign-in didn't use a passkey: offer to make one on this device, if it can.
    AddPasskey,
}

/// A sign-in or refresh that worked: the tokens to hand out and who they are for.
#[derive(Debug, Clone)]
pub struct SignedIn {
    /// The user, as of now.
    pub user: User,
    /// The session.
    pub session_id: Uuid,
    /// The surface.
    pub surface: ApiSurface,
    /// The access token (a JWT).
    pub access_token: String,
    /// How long the access token lasts.
    pub access_expires_in: Duration,
    /// A new refresh token for the cookie, with how long it lasts. `None` when a racing
    /// refresh already replaced it (the browser has the new one).
    pub refresh: Option<(String, Duration)>,
    /// What to offer next.
    pub next_steps: Vec<NextStep>,
}
