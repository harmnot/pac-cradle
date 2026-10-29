//! Response compression. Nginx leaves it to the API (`gzip off` there) because compressing a
//! response that holds a secret next to text an attacker chose can leak the secret (BREACH).
//! Responses that carry a token are sent uncompressed: they set the refresh cookie, are marked
//! `Cache-Control: no-store`, or both (M1-D3).

use axum::body::HttpBody;
use axum::http::Response;
use axum::http::header::{CACHE_CONTROL, SET_COOKIE};
use tower_http::compression::CompressionLayer;
use tower_http::compression::predicate::{And, DefaultPredicate, Predicate};

/// Compresses like tower-http's default (bodies over 32 bytes, not images, not event streams),
/// except responses that set a cookie or say `no-store`.
pub fn layer() -> CompressionLayer<And<DefaultPredicate, NoCookie>> {
    CompressionLayer::new().compress_when(DefaultPredicate::new().and(NoCookie))
}

/// Allows compression only for responses that set no cookie and may be stored (`no-store`
/// marks the ones carrying tokens or account data).
#[derive(Debug, Clone, Copy, Default)]
pub struct NoCookie;

impl Predicate for NoCookie {
    fn should_compress<B>(&self, response: &Response<B>) -> bool
    where
        B: HttpBody,
    {
        let headers = response.headers();
        let no_store = headers
            .get_all(CACHE_CONTROL)
            .iter()
            .filter_map(|value| value.to_str().ok())
            .any(|value| value.to_ascii_lowercase().contains("no-store"));
        !headers.contains_key(SET_COOKIE) && !no_store
    }
}
