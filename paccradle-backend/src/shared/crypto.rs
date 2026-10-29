//! Random secrets and hashes, from `ring` (already built for sqlx's TLS).

use ring::digest;
use ring::rand::{SecureRandom, SystemRandom};

use super::base64url;
use crate::error::{AppError, ErrorCode};

/// Bytes of randomness in a token: 256 bits, so a token can be stored as a plain SHA-256.
pub const TOKEN_BYTES: usize = 32;

/// `N` bytes from the operating system's secure random source.
pub fn random_bytes<const N: usize>() -> Result<[u8; N], AppError> {
    let mut bytes = [0u8; N];
    SystemRandom::new()
        .fill(&mut bytes)
        .map_err(|_| AppError::new(ErrorCode::Internal).with_meta("dependency", "random"))?;
    Ok(bytes)
}

/// SHA-256 of `bytes`.
pub fn sha256(bytes: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    out.copy_from_slice(digest::digest(&digest::SHA256, bytes).as_ref());
    out
}

/// A new secret token (refresh tokens, email links) and the hash to store in its place.
#[derive(Clone)]
pub struct NewToken {
    /// The token, base64url: goes to the user (cookie, email) and is never stored.
    pub token: String,
    /// [`token_hash`] of it: what the database keeps.
    pub hash: [u8; 32],
}

impl NewToken {
    /// A fresh random token.
    pub fn generate() -> Result<Self, AppError> {
        let token = base64url::encode(&random_bytes::<TOKEN_BYTES>()?);
        let hash = token_hash(&token);
        Ok(Self { token, hash })
    }
}

/// What a token is stored as: the SHA-256 of its text. Tokens carry 256 random bits, so no salt
/// or slow hash is needed; a leaked table can't be turned back into tokens.
pub fn token_hash(token: &str) -> [u8; 32] {
    sha256(token.as_bytes())
}
