use uuid::Uuid;

use crate::error::AppError;
use crate::models::passkey::{
    AuthenticationCredential, Passkey, PasskeyOptions, RegistrationCredential,
};
use crate::models::session::{AuthUser, SignedIn};
use crate::shared::api_surface::ApiSurface;
use crate::shared::future::BoxFuture;

/// A browser's answer to a passkey sign-in.
#[derive(Debug, Clone)]
pub struct PasskeySignIn<'a> {
    /// The surface signing in.
    pub surface: ApiSurface,
    /// The ceremony the options started.
    pub ceremony_id: Uuid,
    /// What the authenticator returned.
    pub credential: AuthenticationCredential,
    /// The browser's refresh token, if it had one (see `StartSession::replacing`).
    pub replacing: Option<&'a str>,
}

/// A browser's answer to a passkey registration.
#[derive(Debug, Clone)]
pub struct PasskeyRegistration {
    /// The ceremony the options started.
    pub ceremony_id: Uuid,
    /// What the authenticator returned.
    pub credential: RegistrationCredential,
    /// A name for it, e.g. "MacBook" (may be empty).
    pub name: String,
}

/// Passkey ceremonies. Every refusal is `AUTH_PASSKEY_REJECTED` (the reason is logged), so a
/// caller learns nothing about which passkeys exist.
pub trait PasskeyService: Send + Sync {
    /// Starts a sign-in: a single-use challenge for any passkey on this site.
    fn sign_in_options(
        &self,
        surface: ApiSurface,
    ) -> BoxFuture<'_, Result<PasskeyOptions, AppError>>;

    /// Finishes a sign-in and starts a session. On the admin surface the passkey's owner
    /// must be an admin.
    fn sign_in<'a>(
        &'a self,
        sign_in: PasskeySignIn<'a>,
    ) -> BoxFuture<'a, Result<SignedIn, AppError>>;

    /// Starts adding a passkey to the signed-in account (not a guest).
    fn registration_options<'a>(
        &'a self,
        user: &'a AuthUser,
    ) -> BoxFuture<'a, Result<PasskeyOptions, AppError>>;

    /// Checks and stores the new passkey. `AUTH_PASSKEY_ALREADY_REGISTERED` if it is stored
    /// already.
    fn register<'a>(
        &'a self,
        registration: (&'a AuthUser, PasskeyRegistration),
    ) -> BoxFuture<'a, Result<Passkey, AppError>>;
}
