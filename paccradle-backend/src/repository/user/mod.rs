//! The `users` table.

mod user_repository;
mod user_repository_trait;

pub use user_repository::PgUserRepository;
pub use user_repository_trait::UserRepository;
