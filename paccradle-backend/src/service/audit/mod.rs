//! The log of admin writes (who did what, when).

mod audit_service;
mod audit_service_trait;

pub use audit_service::DefaultAuditService;
pub use audit_service_trait::AuditService;
