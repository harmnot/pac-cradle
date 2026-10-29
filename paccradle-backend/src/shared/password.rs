//! Password hashes: Argon2id with OWASP's parameters (19 MiB, 2 passes, 1 lane), stored as PHC
//! strings so the parameters can be raised later without breaking old hashes. Both functions
//! take tens of milliseconds of CPU: call them from a blocking task.

use argon2::Argon2;
use argon2::password_hash::{PasswordHasher, PasswordVerifier};

use super::crypto::random_bytes;
use crate::error::{AppError, ErrorCode};

/// Bytes of salt per hash.
const SALT_BYTES: usize = 16;

/// A PHC string for a password made only so that a sign-in for an unknown address takes as long
/// as one for a real account. No password matches it in practice: its hash is all zeros.
pub const DUMMY_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$AAAAAAAAAAAAAAAAAAAAAA$AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";

/// The Argon2id PHC string for `password`, with a fresh random salt.
pub fn hash(password: &str) -> Result<String, AppError> {
    let salt = random_bytes::<SALT_BYTES>()?;
    Argon2::default()
        .hash_password_with_salt(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|error| {
            AppError::new(ErrorCode::Internal).with_source(PasswordHashError(error.to_string()))
        })
}

/// Whether `password` matches the PHC string `hash`. A hash that can't be read matches nothing.
pub fn verify(password: &str, hash: &str) -> bool {
    Argon2::default()
        .verify_password(password.as_bytes(), hash)
        .is_ok()
}

/// The hasher's own error, for the logs.
#[derive(Debug, thiserror::Error)]
#[error("cannot hash the password: {0}")]
struct PasswordHashError(String);
