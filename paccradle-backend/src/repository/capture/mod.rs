//! The `captures` table.

mod capture_repository;
mod capture_repository_trait;

pub use capture_repository::PgCaptureRepository;
pub use capture_repository_trait::CaptureRepository;
