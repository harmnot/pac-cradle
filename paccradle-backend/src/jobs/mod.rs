//! The background worker (`backend-architecture.md` §4.9): it claims due jobs, runs each
//! with its kind's handler, and reports back to the job service. Handlers only call services,
//! and must be safe to run twice (a job can run again after its worker dies mid-run).

pub mod handler;
pub mod send_email_link;
pub mod worker;

pub use handler::{JobHandler, JobHandlers};
pub use worker::Worker;

use crate::AppState;

/// The handlers this release runs: sign-in emails (M1-D3). Others join with their task: photo
/// purge (M1-D4), vision (D5), exports (D7), Paddle webhooks (F3).
pub fn handlers(state: &AppState) -> JobHandlers {
    JobHandlers::new().with(send_email_link::SendEmailLinkHandler::new(
        state.identity.clone(),
    ))
}
