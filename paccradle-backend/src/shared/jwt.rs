//! Compact JWTs signed with HMAC-SHA256 (RFC 7519, `HS256` only), for the short-lived access
//! tokens. Only one header is made or accepted, so the algorithm can't be swapped by a caller
//! (`alg: none`, or RS/HS confusion). Expiry and audience are checked by the caller, which owns
//! those rules.

use ring::hmac;
use serde::Serialize;
use serde::de::DeserializeOwned;

use super::base64url;

/// `{"alg":"HS256","typ":"JWT"}`, the only header made or accepted.
const HEADER: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9";

/// The longest token read: access tokens are about 300 characters.
const MAX_TOKEN_LEN: usize = 4096;

/// The signing key, at least 32 bytes.
pub struct JwtKey(hmac::Key);

impl JwtKey {
    /// A key from `secret` bytes.
    pub fn new(secret: &[u8]) -> Self {
        Self(hmac::Key::new(hmac::HMAC_SHA256, secret))
    }
}

/// Why a token was refused. Callers answer every case the same way.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum JwtError {
    /// Not three parts, another header, or base64url or JSON that can't be read.
    #[error("the token is malformed")]
    Malformed,
    /// The signature doesn't match.
    #[error("the token's signature is wrong")]
    BadSignature,
}

/// Signs `claims` into a token.
pub fn encode<C: Serialize>(key: &JwtKey, claims: &C) -> Result<String, serde_json::Error> {
    let payload = base64url::encode(&serde_json::to_vec(claims)?);
    let signing_input = format!("{HEADER}.{payload}");
    let signature = hmac::sign(&key.0, signing_input.as_bytes());
    Ok(format!(
        "{signing_input}.{}",
        base64url::encode(signature.as_ref())
    ))
}

/// The claims of `token` once its header and signature check out (in constant time).
pub fn decode<C: DeserializeOwned>(key: &JwtKey, token: &str) -> Result<C, JwtError> {
    if token.len() > MAX_TOKEN_LEN {
        return Err(JwtError::Malformed);
    }
    let (signing_input, signature) = token.rsplit_once('.').ok_or(JwtError::Malformed)?;
    let (header, payload) = signing_input.split_once('.').ok_or(JwtError::Malformed)?;
    if header != HEADER {
        return Err(JwtError::Malformed);
    }
    let signature = base64url::decode(signature).ok_or(JwtError::Malformed)?;
    hmac::verify(&key.0, signing_input.as_bytes(), &signature)
        .map_err(|_| JwtError::BadSignature)?;
    let payload = base64url::decode(payload).ok_or(JwtError::Malformed)?;
    serde_json::from_slice(&payload).map_err(|_| JwtError::Malformed)
}
