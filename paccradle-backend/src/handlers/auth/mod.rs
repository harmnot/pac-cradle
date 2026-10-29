//! Sign-in routes both surfaces have: passkey sign-in, refresh and sign-out. Each surface
//! mounts them under its own prefix (`/api/{surface}/v1/auth/…`) with its own refresh cookie,
//! web origin, sign-in rate limit and token audience.

pub mod auth_handler;
pub mod auth_route;
pub mod refresh_cookie;
