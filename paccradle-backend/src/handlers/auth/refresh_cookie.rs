//! The refresh token's cookie: `HttpOnly` (scripts never see it), `SameSite=Strict`, `Secure`
//! on HTTPS, and scoped to the surface's `/auth` routes, the only ones that read it.

use std::time::Duration;

use axum::http::header::COOKIE;
use axum::http::{HeaderMap, HeaderValue};

use crate::config::AppConfig;
use crate::shared::api_surface::ApiSurface;

/// The cookie's name on `surface`.
pub const fn name(surface: ApiSurface) -> &'static str {
    match surface {
        ApiSurface::Client => "paccradle_refresh",
        ApiSurface::Admin => "paccradle_admin_refresh",
    }
}

/// The path it is sent to, e.g. `/api/client/v1/auth`.
pub fn path(surface: ApiSurface) -> String {
    format!("{}/auth", surface.prefix())
}

/// The refresh token in the request's cookies, if any.
pub fn read(headers: &HeaderMap, surface: ApiSurface) -> Option<String> {
    let wanted = name(surface);
    headers
        .get_all(COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(name, value)| *name == wanted && !value.is_empty())
        .map(|(_, value)| value.to_owned())
}

/// Whether the cookie gets `Secure`: always, except for a web app on plain `http://`
/// (localhost in development), where browsers would drop it.
pub fn secure(config: &AppConfig, surface: ApiSurface) -> bool {
    let origin = match surface {
        ApiSurface::Client => Some(&config.web_origin),
        ApiSurface::Admin => config.admin_web_origin.as_ref(),
    };
    !origin.is_some_and(|origin| origin.as_bytes().starts_with(b"http://"))
}

/// `Set-Cookie` for `token`, lasting `lasts`.
pub fn set(surface: ApiSurface, (token, lasts): (&str, Duration), secure: bool) -> HeaderValue {
    cookie(surface, (token, lasts.as_secs()), secure)
}

/// `Set-Cookie` that deletes the cookie.
pub fn clear(surface: ApiSurface, secure: bool) -> HeaderValue {
    cookie(surface, ("", 0), secure)
}

fn cookie(surface: ApiSurface, (value, max_age): (&str, u64), secure: bool) -> HeaderValue {
    let secure = if secure { "; Secure" } else { "" };
    let text = format!(
        "{}={value}; Path={}; Max-Age={max_age}; HttpOnly; SameSite=Strict{secure}",
        name(surface),
        path(surface),
    );
    // Tokens are base64url, so the value is always a valid header.
    HeaderValue::from_str(&text).unwrap_or_else(|_| HeaderValue::from_static(""))
}
