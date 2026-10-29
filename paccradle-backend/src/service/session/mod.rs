//! Sessions: access tokens, rotating refresh tokens, and what a sign-in replaces.

mod session_service;
mod session_service_trait;

pub use session_service::{
    ADMIN_ACCESS_TTL, ADMIN_SESSION_TTL, CLIENT_ACCESS_TTL, CLIENT_SESSION_TTL,
    DefaultSessionService, REFRESH_RACE_GRACE,
};
pub use session_service_trait::{SessionService, StartSession};
