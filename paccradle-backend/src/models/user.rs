//! People using the app (the `users` table).

use serde::Serialize;
use time::OffsetDateTime;
use uuid::Uuid;

use super::TextEnum;

/// `users.role`: what a signed-in person may do beyond their own data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// Everyone.
    User,
    /// May sign in to the back office (with a passkey).
    Admin,
}

impl TextEnum for Role {
    const ALL: &'static [Self] = &[Self::User, Self::Admin];

    fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Admin => "admin",
        }
    }
}

/// What a session acts as: a guest (no email yet), a user, or an admin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountRole {
    /// A visitor who hasn't confirmed an email: may capture and design.
    Guest,
    /// A person with a confirmed email.
    User,
    /// A person with a confirmed email and the admin role.
    Admin,
}

impl TextEnum for AccountRole {
    const ALL: &'static [Self] = &[Self::Guest, Self::User, Self::Admin];

    fn as_str(self) -> &'static str {
        match self {
            Self::Guest => "guest",
            Self::User => "user",
            Self::Admin => "admin",
        }
    }
}

/// `users.paper_default`: the paper the capture screen starts with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PaperDefault {
    /// 210 × 297 mm.
    A4,
    /// 8.5 × 11 in.
    Letter,
}

impl TextEnum for PaperDefault {
    const ALL: &'static [Self] = &[Self::A4, Self::Letter];

    fn as_str(self) -> &'static str {
        match self {
            Self::A4 => "a4",
            Self::Letter => "letter",
        }
    }
}

/// A user as the app sees them. The password hash is read only where it is checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    /// The user's id; also the user handle their passkeys carry.
    pub id: Uuid,
    /// The confirmed email; `None` for a guest.
    pub email: Option<String>,
    /// Their role.
    pub role: Role,
    /// Their language, e.g. `en`.
    pub locale: String,
    /// The paper the capture screen starts with.
    pub paper_default: PaperDefault,
    /// Whether they have set a password.
    pub has_password: bool,
    /// How many passkeys they have.
    pub passkey_count: i64,
    /// When the row was made (for a guest who signed up: when they first came).
    pub created_at: OffsetDateTime,
}

impl User {
    /// Whether this is a guest (no confirmed email).
    pub fn is_guest(&self) -> bool {
        self.email.is_none()
    }

    /// What their sessions act as.
    pub fn account_role(&self) -> AccountRole {
        match (&self.email, self.role) {
            (None, _) => AccountRole::Guest,
            (Some(_), Role::User) => AccountRole::User,
            (Some(_), Role::Admin) => AccountRole::Admin,
        }
    }
}

/// A user row to add: a guest (no email) or an account for a just-confirmed email.
#[derive(Debug, Clone)]
pub struct NewUser {
    /// The id it will have.
    pub id: Uuid,
    /// The confirmed email, or `None` for a guest.
    pub email: Option<String>,
    /// Their language.
    pub locale: String,
}
