//! The `jobs` table.

mod job_repository;
mod job_repository_trait;

pub use job_repository::PgJobRepository;
pub use job_repository_trait::JobRepository;
