//! The `designs` table.

mod design_repository;
mod design_repository_trait;

pub use design_repository::PgDesignRepository;
pub use design_repository_trait::DesignRepository;
