//! Email links (the `email_links` table) and the job that sends them.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use super::TextEnum;
use super::job::JobKind;

/// What an email link is for. The link does the same either way (confirm the address, then
/// sign in to its account, making it if needed); the purpose picks the email's words and how
/// long the link lasts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmailLinkPurpose {
    /// No account has the address yet: the link confirms it and makes one.
    SignUp,
    /// An account has it: the link signs in (also "forgot password").
    SignIn,
}

impl TextEnum for EmailLinkPurpose {
    const ALL: &'static [Self] = &[Self::SignUp, Self::SignIn];

    fn as_str(self) -> &'static str {
        match self {
            Self::SignUp => "sign_up",
            Self::SignIn => "sign_in",
        }
    }
}

/// An email link as stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmailLink {
    /// The link's id (not its token).
    pub id: Uuid,
    /// The address it goes to, as typed.
    pub email: String,
    /// What it is for.
    pub purpose: EmailLinkPurpose,
    /// The email's language tag.
    pub locale: String,
    /// When it was asked for.
    pub created_at: OffsetDateTime,
    /// Whether it was used (or retired by another link's use).
    pub used: bool,
}

/// An email link to add; the send job makes its token.
#[derive(Debug, Clone)]
pub struct NewEmailLink {
    /// The id it will have.
    pub id: Uuid,
    /// The address, as typed.
    pub email: String,
    /// What it is for.
    pub purpose: EmailLinkPurpose,
    /// The email's language tag.
    pub locale: String,
}

/// The job that makes a link's token and emails it.
pub struct SendEmailLink;

/// [`SendEmailLink`]'s payload: only the link's id, so no token or address sits in the queue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendEmailLinkPayload {
    /// The `email_links` row.
    pub link_id: Uuid,
}

impl JobKind for SendEmailLink {
    const NAME: &'static str = "send_email_link";
    type Payload = SendEmailLinkPayload;
}
