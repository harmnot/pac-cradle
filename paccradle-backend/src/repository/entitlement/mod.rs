//! `design_unlocks`, `credit_ledger` and `subscriptions`: what a user may export.

mod entitlement_repository;
mod entitlement_repository_trait;

pub use entitlement_repository::PgEntitlementRepository;
pub use entitlement_repository_trait::EntitlementRepository;
