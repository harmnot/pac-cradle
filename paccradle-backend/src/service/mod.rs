//! Business logic (`backend-architecture.md` §4.5), one folder per domain, each behind a trait.

pub mod audit;
pub mod billing;
pub mod capture;
pub mod design;
pub mod export;
pub mod health;
pub mod identity;
pub mod job;
pub mod passkey;
pub mod privacy;
pub mod session;
