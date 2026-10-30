//! People's data: the 30-day photo purge (M1-D4); account deletion and data export later.

mod privacy_service;
mod privacy_service_trait;

pub use privacy_service::{
    DefaultPrivacyService, PURGE_BATCH, PURGE_TIME_BUDGET, PrivacyProviders,
};
pub use privacy_service_trait::{PrivacyService, PurgeReport};
