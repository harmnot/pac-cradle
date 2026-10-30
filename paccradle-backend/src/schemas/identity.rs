//! Sign-in and account bodies (M1-D3). Request bodies holding a password or a token have no
//! `Debug`, so they can't reach a log line.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{AppError, ErrorCode};
use crate::models::paper::Paper;
use crate::models::session::{NextStep, SignedIn};
use crate::models::user::{AccountRole, User};

/// The longest email address (RFC 5321's path limit).
const MAX_EMAIL_LEN: usize = 254;
/// The longest password read on sign-in, in bytes; longer ones can't have been set.
const MAX_PASSWORD_BYTES: usize = 1024;
/// The longest email-link token read (they are 43 characters).
const MAX_TOKEN_LEN: usize = 128;

/// `POST /auth/email-link`: send a sign-up or sign-in link.
#[derive(Deserialize)]
pub struct EmailLinkRequest {
    /// The address.
    pub email: String,
}

impl EmailLinkRequest {
    /// The address, trimmed and shape-checked.
    pub fn email(&self) -> Result<&str, AppError> {
        valid_email(&self.email)
    }
}

/// `POST /auth/email-link/sign-in`: the token from the link's `#token=` fragment.
#[derive(Deserialize)]
pub struct EmailLinkSignInRequest {
    /// The token.
    pub token: String,
}

impl EmailLinkSignInRequest {
    /// The token, if its length is possible (otherwise `AUTH_LINK_INVALID`, like any bad one).
    pub fn token(&self) -> Result<&str, AppError> {
        let token = self.token.trim();
        if token.is_empty() || token.len() > MAX_TOKEN_LEN {
            return Err(AppError::new(ErrorCode::AuthLinkInvalid));
        }
        Ok(token)
    }
}

/// `POST /auth/password/sign-in`.
#[derive(Deserialize)]
pub struct PasswordSignInRequest {
    /// The account's email.
    pub email: String,
    /// The password.
    pub password: String,
}

impl PasswordSignInRequest {
    /// The address and password, shape-checked.
    pub fn parts(&self) -> Result<(&str, &str), AppError> {
        let email = valid_email(&self.email)?;
        if self.password.is_empty() || self.password.len() > MAX_PASSWORD_BYTES {
            return Err(invalid("password"));
        }
        Ok((email, &self.password))
    }
}

/// `PUT /me/password`.
#[derive(Deserialize)]
pub struct SetPasswordRequest {
    /// The new password, 12 to 128 characters.
    pub password: String,
    /// The current password; leave it out within 15 minutes of a sign-in by email link or
    /// passkey (how "forgot password" works).
    #[serde(default)]
    pub current_password: Option<String>,
}

/// A session's tokens: every sign-in and refresh answers with this, and sets the refresh
/// cookie (HttpOnly, so the web app never sees that token).
#[derive(Debug, Serialize)]
pub struct SessionResponse {
    /// Send it as `Authorization: Bearer <access_token>`.
    pub access_token: String,
    /// Always `Bearer`.
    pub token_type: &'static str,
    /// Seconds until the access token expires; refresh a little before.
    pub expires_in: u64,
    /// Who is signed in.
    pub account: AccountResponse,
    /// What to offer right after a sign-in (empty after a refresh).
    pub next_steps: Vec<NextStep>,
}

impl From<SignedIn> for SessionResponse {
    fn from(signed_in: SignedIn) -> Self {
        Self {
            access_token: signed_in.access_token,
            token_type: "Bearer",
            expires_in: signed_in.access_expires_in.as_secs(),
            account: AccountResponse::from(signed_in.user),
            next_steps: signed_in.next_steps,
        }
    }
}

/// `GET /me`, and the account in a [`SessionResponse`].
#[derive(Debug, Serialize)]
pub struct AccountResponse {
    /// The user's id.
    pub id: Uuid,
    /// Their confirmed email; `null` for a guest.
    pub email: Option<String>,
    /// `guest`, `user` or `admin`.
    pub role: AccountRole,
    /// Whether a password is set.
    pub has_password: bool,
    /// How many passkeys they have.
    pub passkey_count: i64,
    /// Their language, e.g. `en`.
    pub locale: String,
    /// `a4` or `letter`.
    pub paper_default: Paper,
    /// When the account (or the guest it grew from) was made, RFC 3339.
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

impl From<User> for AccountResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            role: user.account_role(),
            email: user.email,
            has_password: user.has_password,
            passkey_count: user.passkey_count,
            locale: user.locale,
            paper_default: user.paper_default,
            created_at: user.created_at,
        }
    }
}

/// `email` trimmed, if it looks like an address: something, `@`, a domain with a dot, no
/// spaces, at most 254 characters. The email itself is the real check.
fn valid_email(email: &str) -> Result<&str, AppError> {
    let email = email.trim();
    let Some((local, domain)) = email.rsplit_once('@') else {
        return Err(invalid("email"));
    };
    let domain_ok = domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && !domain.contains("..");
    let ok = email.len() <= MAX_EMAIL_LEN
        && !local.is_empty()
        && domain_ok
        && !email.chars().any(|c| c.is_whitespace() || c.is_control());
    match ok {
        true => Ok(email),
        false => Err(invalid("email")),
    }
}

/// `INVALID_REQUEST` naming `field`.
pub(crate) fn invalid(field: &str) -> AppError {
    AppError::new(ErrorCode::InvalidRequest).with_meta("field", field)
}
