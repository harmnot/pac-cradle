//! Accounts on the web app: guests, email links (sign-up and sign-in), passwords, `/me`.

mod identity_service;
mod identity_service_trait;

pub use identity_service::{
    DefaultIdentityService, EMAIL_LINK_WINDOW, EMAIL_LINKS_PER_WINDOW, FRESH_SIGN_IN,
    IdentityProviders, MAX_PASSWORD_CHARS, MIN_PASSWORD_CHARS, SIGN_IN_LINK_TTL, SIGN_UP_LINK_TTL,
};
pub use identity_service_trait::{EmailLinkRequest, IdentityService, PasswordSignIn, SetPassword};
