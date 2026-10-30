use std::sync::Arc;

use super::JobHandler;
use super::send_email_link::job_error;
use crate::models::capture::{PurgePhotos, PurgePhotosPayload};
use crate::models::job::JobError;
use crate::service::privacy::PrivacyService;

/// Deletes the photos past their 30 days (`purge_photos`, daily), through the privacy service.
pub struct PurgePhotosHandler {
    privacy: Arc<dyn PrivacyService>,
}

impl PurgePhotosHandler {
    /// A handler purging through `privacy`.
    pub fn new(privacy: Arc<dyn PrivacyService>) -> Self {
        Self { privacy }
    }
}

impl JobHandler for PurgePhotosHandler {
    type Kind = PurgePhotos;

    async fn run(&self, _: PurgePhotosPayload) -> Result<(), JobError> {
        let report = self.privacy.purge_due_photos().await.map_err(job_error)?;
        if report.deleted > 0 || report.continued {
            tracing::info!(
                deleted = report.deleted,
                continued = report.continued,
                "photos past their 30 days deleted"
            );
        }
        Ok(())
    }
}
