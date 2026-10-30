use std::sync::Arc;

use super::JobHandler;
use super::send_email_link::job_error;
use crate::models::capture::{RunVision, RunVisionPayload};
use crate::models::job::JobError;
use crate::service::capture::CaptureService;

/// Measures a capture's photo (`run_vision`), through the capture service. Storage and
/// database outages are retried; whatever vision says is saved with the capture. A run whose
/// job fails for good is marked failed, so the capture doesn't stay pending.
pub struct RunVisionHandler {
    captures: Arc<dyn CaptureService>,
}

impl RunVisionHandler {
    /// A handler measuring through `captures`.
    pub fn new(captures: Arc<dyn CaptureService>) -> Self {
        Self { captures }
    }
}

impl JobHandler for RunVisionHandler {
    type Kind = RunVision;

    async fn run(&self, payload: RunVisionPayload) -> Result<(), JobError> {
        self.captures.run_vision(payload).await.map_err(job_error)
    }

    async fn failed_for_good(&self, payload: RunVisionPayload) {
        if let Err(error) = self.captures.give_up_run(payload).await {
            tracing::error!(
                capture_id = %payload.capture_id,
                run = payload.run,
                %error,
                "cannot mark a capture's failed run"
            );
        }
    }
}
