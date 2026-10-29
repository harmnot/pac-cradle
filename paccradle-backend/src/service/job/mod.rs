//! The job queue: adding jobs with a service's own writes, and the rules for a claimed job's
//! lease, retries and dead letters.

mod job_service;
mod job_service_trait;

pub use job_service::{DefaultJobService, RetryPolicy};
pub use job_service_trait::JobService;
