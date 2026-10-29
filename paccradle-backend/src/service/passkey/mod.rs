//! Passkeys: sign-in on both surfaces, registration on the web app.

mod passkey_service;
mod passkey_service_trait;

pub use passkey_service::{CEREMONY_TTL, DefaultPasskeyService, PasskeyProviders};
pub use passkey_service_trait::{PasskeyRegistration, PasskeyService, PasskeySignIn};
