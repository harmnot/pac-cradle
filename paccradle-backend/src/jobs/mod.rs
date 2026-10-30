//! The background worker (`backend-architecture.md` §4.9): it claims due jobs, runs each
//! with its kind's handler, and reports back to the job service. Handlers only call services,
//! and must be safe to run twice (a job can run again after its worker dies mid-run).

pub mod handler;
pub mod purge_photos;
pub mod run_vision;
pub mod send_email_link;
pub mod worker;

use std::time::Duration;

pub use handler::{JobHandler, JobHandlers};
pub use worker::Worker;

use crate::AppState;
use crate::error::AppError;
use crate::models::capture::{PurgePhotos, PurgePhotosPayload};
use crate::models::job::Schedule;

/// A day, for daily jobs.
const DAY: Duration = Duration::from_secs(24 * 60 * 60);

/// The handlers this release runs: sign-in emails (M1-D3), the photo purge (M1-D4) and vision
/// runs (M1-D5). Others join with their task: exports (D7), Paddle webhooks (F3).
pub fn handlers(state: &AppState) -> JobHandlers {
    JobHandlers::new()
        .with(send_email_link::SendEmailLinkHandler::new(
            state.identity.clone(),
        ))
        .with(purge_photos::PurgePhotosHandler::new(state.privacy.clone()))
        .with(run_vision::RunVisionHandler::new(state.captures.clone()))
}

/// The jobs that run by themselves: the photo purge, a day after the last run finished (and
/// at the first start). The billing repair job (M1-F) joins here.
pub fn schedules() -> Result<Vec<Schedule>, AppError> {
    Ok(vec![Schedule::of::<PurgePhotos>((
        &PurgePhotosPayload {},
        DAY,
    ))?])
}
