//! The `passkeys` and `passkey_challenges` tables.

mod passkey_repository;
mod passkey_repository_trait;

pub use passkey_repository::PgPasskeyRepository;
pub use passkey_repository_trait::PasskeyRepository;
