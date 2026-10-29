//! The `admin_audit_log` table.

mod audit_repository;
mod audit_repository_trait;

pub use audit_repository::PgAuditRepository;
pub use audit_repository_trait::AuditRepository;
