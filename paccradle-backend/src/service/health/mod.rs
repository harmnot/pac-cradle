//! Whether the API can serve requests (`/readyz`).

mod health_service;
mod health_service_trait;

pub use health_service::DefaultHealthService;
pub use health_service_trait::HealthService;
