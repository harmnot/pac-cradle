//! Whether the database answers.

mod health_repository;
mod health_repository_trait;

pub use health_repository::PgHealthRepository;
pub use health_repository_trait::HealthRepository;
