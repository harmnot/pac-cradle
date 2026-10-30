use std::sync::Arc;
use std::time::{Duration, Instant};

use super::{PrivacyService, PurgeReport};
use crate::error::AppError;
use crate::models::capture::{PurgePhotos, PurgePhotosPayload};
use crate::models::job::NewJob;
use crate::repository::capture::CaptureRepository;
use crate::repository::database::Database;
use crate::service::job::JobService;
use crate::shared::future::BoxFuture;
use crate::third_party::storage::ObjectStorage;

/// Photos looked up at a time.
pub const PURGE_BATCH: i64 = 100;
/// How long one purge run goes on before it hands over to the next: well inside a job's
/// 5-minute lease.
pub const PURGE_TIME_BUDGET: Duration = Duration::from_secs(3 * 60);

/// What [`DefaultPrivacyService`] works with.
pub struct PrivacyProviders {
    /// Connections and transactions.
    pub db: Arc<dyn Database>,
    /// `captures`.
    pub captures: Arc<dyn CaptureRepository>,
    /// Where photos are kept.
    pub storage: Arc<dyn ObjectStorage>,
    /// The job queue, for a run that hands over.
    pub jobs: Arc<dyn JobService>,
}

/// [`PrivacyService`] on Postgres and object storage.
pub struct DefaultPrivacyService {
    parts: PrivacyProviders,
    time_budget: Duration,
}

impl DefaultPrivacyService {
    /// A service on `parts`.
    pub fn new(parts: PrivacyProviders) -> Self {
        Self {
            parts,
            time_budget: PURGE_TIME_BUDGET,
        }
    }

    /// The same service, handing over after `time_budget` (tests).
    pub fn with_time_budget(mut self, time_budget: Duration) -> Self {
        self.time_budget = time_budget;
        self
    }

    /// Queues another purge run, due now.
    async fn hand_over(&self) -> Result<(), AppError> {
        let mut tx = self.parts.db.begin().await?;
        let job = NewJob::of::<PurgePhotos>(&PurgePhotosPayload {})?;
        self.parts.jobs.enqueue(&mut tx, &job).await?;
        tx.commit().await?;
        Ok(())
    }
}

impl PrivacyService for DefaultPrivacyService {
    fn purge_due_photos(&self) -> BoxFuture<'_, Result<PurgeReport, AppError>> {
        Box::pin(async move {
            let started = Instant::now();
            let mut report = PurgeReport::default();
            loop {
                let due = {
                    let mut conn = self.parts.db.connection().await?;
                    self.parts
                        .captures
                        .photos_due(&mut conn, PURGE_BATCH)
                        .await?
                };
                for photo in &due {
                    self.parts.storage.delete(&photo.storage_key).await?;
                    let mut conn = self.parts.db.connection().await?;
                    if self.parts.captures.forget_photo(&mut conn, photo).await? {
                        report.deleted += 1;
                    }
                }
                if due.len() < usize::try_from(PURGE_BATCH).unwrap_or(usize::MAX) {
                    return Ok(report);
                }
                if started.elapsed() >= self.time_budget {
                    self.hand_over().await?;
                    report.continued = true;
                    return Ok(report);
                }
            }
        })
    }
}
