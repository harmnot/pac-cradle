use uuid::Uuid;

use crate::error::AppError;
use crate::locales::Locale;
use crate::models::session::{AuthUser, SignedIn};
use crate::models::user::User;
use crate::shared::future::BoxFuture;

/// A request for an email link.
#[derive(Debug, Clone, Copy)]
pub struct EmailLinkRequest<'a> {
    /// The address, trimmed and shape-checked by the handler.
    pub email: &'a str,
    /// The language to write the email in.
    pub locale: Locale,
}

/// A password sign-in. No `Debug`: it holds the password.
#[derive(Clone, Copy)]
pub struct PasswordSignIn<'a> {
    /// The account's email.
    pub email: &'a str,
    /// The password typed.
    pub password: &'a str,
    /// The browser's refresh token, if it had one (see `StartSession::replacing`).
    pub replacing: Option<&'a str>,
}

/// A new password. No `Debug`: it holds passwords.
#[derive(Clone, Copy)]
pub struct SetPassword<'a> {
    /// The new password.
    pub password: &'a str,
    /// The current one; not needed right after a sign-in by email link or passkey.
    pub current_password: Option<&'a str>,
}

/// Web app accounts: guests, email links, passwords.
pub trait IdentityService: Send + Sync {
    /// Makes a guest and signs them in, so a visitor can capture and design at once.
    fn start_guest(&self, locale: Locale) -> BoxFuture<'_, Result<SignedIn, AppError>>;

    /// Queues an email link: a sign-in link when an account has the address, otherwise one
    /// that confirms it and makes the account. The answer is the same either way, so it
    /// tells nobody whether an address has an account. `RATE_LIMITED` after a few for one
    /// address.
    fn request_email_link<'a>(
        &'a self,
        request: EmailLinkRequest<'a>,
    ) -> BoxFuture<'a, Result<(), AppError>>;

    /// Makes the link's token and emails it (the `send_email_link` job). A used or deleted
    /// link is skipped. Each run makes a new token, so only the last email's link works.
    fn deliver_email_link(&self, link_id: Uuid) -> BoxFuture<'_, Result<(), AppError>>;

    /// Signs in with an email link's token: confirms the address, makes the account if it has
    /// none (from the browser's guest, if it is one), and starts a session. Every other unused
    /// link for the address stops working. `AUTH_LINK_INVALID` for an unknown, used or expired
    /// token.
    fn sign_in_with_email_link<'a>(
        &'a self,
        link: (&'a str, Option<&'a str>),
    ) -> BoxFuture<'a, Result<SignedIn, AppError>>;

    /// Signs in with email and password. `AUTH_INVALID_CREDENTIALS` for any mismatch, in the
    /// same time whether or not the account exists; `RATE_LIMITED` after a few tries a minute
    /// for one address.
    fn sign_in_with_password<'a>(
        &'a self,
        sign_in: PasswordSignIn<'a>,
    ) -> BoxFuture<'a, Result<SignedIn, AppError>>;

    /// The signed-in user.
    fn account<'a>(&'a self, user: &'a AuthUser) -> BoxFuture<'a, Result<User, AppError>>;

    /// Sets or changes the password, then ends the user's other sessions. Needs the current
    /// password, or a sign-in by email link or passkey within the last few minutes.
    fn set_password<'a>(
        &'a self,
        change: (&'a AuthUser, SetPassword<'a>),
    ) -> BoxFuture<'a, Result<(), AppError>>;
}
