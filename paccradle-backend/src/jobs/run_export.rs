use std::sync::Arc;

use super::JobHandler;
use super::send_email_link::job_error;
use crate::models::export::{RunExport, RunExportPayload};
use crate::models::job::JobError;
use crate::service::export::ExportService;

/// Makes an export's files (`run_export`), through the export service. Storage and database
/// outages are retried; an export whose job fails for good is marked failed, and asking
/// again costs nothing (its design stays unlocked).
pub struct RunExportHandler {
    exports: Arc<dyn ExportService>,
}

impl RunExportHandler {
    /// A handler exporting through `exports`.
    pub fn new(exports: Arc<dyn ExportService>) -> Self {
        Self { exports }
    }
}

impl JobHandler for RunExportHandler {
    type Kind = RunExport;

    async fn run(&self, payload: RunExportPayload) -> Result<(), JobError> {
        self.exports.run(payload).await.map_err(job_error)
    }

    async fn failed_for_good(&self, payload: RunExportPayload) {
        if let Err(error) = self.exports.give_up(payload).await {
            tracing::error!(export_id = %payload.export_id, %error, "cannot mark an export failed");
        }
    }
}
