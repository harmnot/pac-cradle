use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::{Duration, Instant};

use photo_file::PhotoError;
use time::OffsetDateTime;
use tokio::sync::Semaphore;
use uuid::Uuid;
use vision::result::Level;
use vision::{PaperSize, VisionError, VisionResult};

use super::{CaptureService, CorrectionRequest, PhotoUpload};
use crate::error::{AppError, ErrorCode};
use crate::models::capture::{
    Capture, CaptureMode, CaptureSettings, CaptureStatus, CaptureView, Corrected, Correction,
    CreatedCapture, NewCapture, PHOTO_RETENTION, RunVision, RunVisionPayload, StoredError,
    VisionLevel,
};
use crate::models::job::NewJob;
use crate::models::paper::Paper;
use crate::models::session::AuthUser;
use crate::repository::capture::CaptureRepository;
use crate::repository::database::{Database, Tx};
use crate::service::job::JobService;
use crate::shared::future::BoxFuture;
use crate::shared::id::new_id;
use crate::shared::rate_limiter::RateLimiter;
use crate::third_party::storage::{NewObject, ObjectStorage};

/// The largest photo file accepted, in MB (CLAUDE.md §8.5). The browser sends JPEGs of 2 to
/// 6 MB (M1-E2); this leaves room for a PNG or a photo sent straight from a phone. The pixel
/// cap, not the file size, bounds what decoding costs.
pub const MAX_PHOTO_MB: usize = 15;
/// [`MAX_PHOTO_MB`] in bytes.
pub const MAX_PHOTO_BYTES: usize = MAX_PHOTO_MB * 1024 * 1024;
/// How long a signed photo URL works (the editor draws outlines over the photo).
pub const PHOTO_URL_TTL: Duration = Duration::from_secs(10 * 60);
/// Vision runs at once in this process: each holds a decoded photo (up to 40 MP, 120 MB)
/// and a CPU for about half a second.
pub const VISION_AT_ONCE: usize = 2;
/// The window of the per-user vision-run limit.
const RUN_WINDOW: Duration = Duration::from_secs(60 * 60);

/// What [`DefaultCaptureService`] works with.
pub struct CaptureProviders {
    /// Transactions.
    pub db: Arc<dyn Database>,
    /// `captures`.
    pub captures: Arc<dyn CaptureRepository>,
    /// Where photos are kept.
    pub storage: Arc<dyn ObjectStorage>,
    /// The job queue (vision runs go through it).
    pub jobs: Arc<dyn JobService>,
}

/// [`CaptureService`] on Postgres, object storage, the job queue and the vision engine.
pub struct DefaultCaptureService {
    parts: CaptureProviders,
    vision_runs: RateLimiter<Uuid>,
    vision_slots: Semaphore,
}

/// How a vision run ended.
enum RunOutcome {
    /// Vision measured the photo.
    Measured(Box<VisionResult>),
    /// It couldn't, for this reason.
    Failed(AppError),
}

impl DefaultCaptureService {
    /// A service on `parts`, letting each user start `runs_per_hour` vision runs an hour
    /// (uploads and taps).
    pub fn new((parts, runs_per_hour): (CaptureProviders, NonZeroU32)) -> Self {
        Self {
            parts,
            vision_runs: RateLimiter::with_window((runs_per_hour, RUN_WINDOW)),
            vision_slots: Semaphore::new(VISION_AT_ONCE),
        }
    }

    /// Spends one of `user`'s hourly vision runs.
    fn spend_run(&self, user: &AuthUser) -> Result<(), AppError> {
        self.vision_runs
            .check(user.user_id, Instant::now())
            .map_err(|wait| {
                AppError::new(ErrorCode::RateLimited)
                    .with_meta("retry_after_seconds", wait.as_secs().max(1))
            })
    }

    /// Adds the capture row and queues its first run, together.
    async fn insert(&self, capture: &NewCapture) -> Result<Capture, AppError> {
        let mut tx = self.parts.db.begin().await?;
        let stored = self.parts.captures.insert(&mut tx, capture).await?;
        self.queue_run(&mut tx, &stored).await?;
        tx.commit().await?;
        Ok(stored)
    }

    /// Queues the job for `capture`'s current run, in `tx`.
    async fn queue_run(&self, tx: &mut Tx, capture: &Capture) -> Result<(), AppError> {
        let job = NewJob::of::<RunVision>(&RunVisionPayload {
            capture_id: capture.id,
            run: capture.run,
        })?;
        self.parts.jobs.enqueue(tx, &job).await
    }

    /// The caller's capture, locked in `tx`.
    async fn lock_own(
        &self,
        (tx, user, id): (&mut Tx, &AuthUser, Uuid),
    ) -> Result<Capture, AppError> {
        let capture = self.parts.captures.lock(tx, id).await?;
        owned((capture, user))
    }

    /// The capture with a fresh link to its photo, while the photo is kept.
    fn view(&self, capture: Capture) -> Result<CaptureView, AppError> {
        let photo_url = match &capture.storage_key {
            Some(key) => Some((
                self.parts.storage.signed_url((key, PHOTO_URL_TTL))?,
                OffsetDateTime::now_utc() + PHOTO_URL_TTL,
            )),
            None => None,
        };
        Ok(CaptureView { capture, photo_url })
    }

    /// Applies `correction` to the locked `capture` and says whether a run was queued.
    async fn apply(
        &self,
        (tx, capture, correction): (&mut Tx, &mut Capture, Correction),
    ) -> Result<Corrected, AppError> {
        match correction {
            Correction::Tap(point) => {
                if capture.storage_key.is_none() {
                    return Err(AppError::new(ErrorCode::CapturePhotoDeleted));
                }
                capture.settings.tap_point_px = Some(point);
                check_settings(&capture.settings)?;
                start_run(capture);
                self.queue_run(tx, capture).await?;
                Ok(Corrected::Running)
            }
            Correction::Edit(outline) => {
                let previous = stored_result(capture)?
                    .ok_or_else(|| AppError::new(ErrorCode::CaptureNotReady))?;
                let edited = blocking(move || vision::accept_edit((&previous, outline.as_slice())))
                    .await?
                    .map_err(|error| vision_error(&error))?;
                save_result((capture, &edited))?;
                // A pending run would measure the photo again over the user's outline.
                capture.run += 1;
                Ok(Corrected::Done)
            }
            Correction::Height(height_mm) => {
                capture.settings.height_mm = Some(height_mm);
                check_settings(&capture.settings)?;
                if capture.status == CaptureStatus::Pending {
                    // The pending run may already have read the settings without it.
                    start_run(capture);
                    self.queue_run(tx, capture).await?;
                    return Ok(Corrected::Running);
                }
                if let Some(previous) = stored_result(capture)? {
                    let result = vision::set_height((&previous, height_mm))
                        .map_err(|error| vision_error(&error))?;
                    capture.result = Some(to_json(&result)?);
                }
                Ok(Corrected::Done)
            }
        }
    }

    /// Runs vision on `photo`, a few at a time, off the async threads.
    async fn measure(
        &self,
        (input, photo): (vision::CaptureInput, Vec<u8>),
    ) -> Result<Result<VisionResult, VisionError>, AppError> {
        let _slot = self.vision_slots.acquire().await.map_err(internal)?;
        blocking(move || vision::run((&input, &photo))).await
    }

    /// Saves how `run` ended, if it is still the capture's pending run.
    async fn finish_run(
        &self,
        (run, outcome): (RunVisionPayload, RunOutcome),
    ) -> Result<(), AppError> {
        let mut tx = self.parts.db.begin().await?;
        let Some(mut capture) = self.parts.captures.lock(&mut tx, run.capture_id).await? else {
            return Ok(());
        };
        if capture.run != run.run || capture.status != CaptureStatus::Pending {
            return Ok(());
        }
        match outcome {
            RunOutcome::Measured(result) => save_result((&mut capture, &result))?,
            RunOutcome::Failed(error) => {
                capture.status = CaptureStatus::Failed;
                capture.error = Some(StoredError {
                    code: error.code().as_str().to_owned(),
                    meta: error.meta().clone(),
                });
            }
        }
        self.parts.captures.save(&mut tx, &capture).await?;
        tx.commit().await?;
        Ok(())
    }
}

impl CaptureService for DefaultCaptureService {
    fn create(&self, upload: PhotoUpload) -> BoxFuture<'_, Result<CreatedCapture, AppError>> {
        Box::pin(async move {
            let PhotoUpload {
                user,
                settings,
                exif,
                photo,
            } = upload;
            self.spend_run(&user)?;
            check_settings(&settings)?;
            if photo.len() > MAX_PHOTO_BYTES {
                return Err(file_too_large());
            }
            // Walking a 15 MB file takes milliseconds, but it's CPU work all the same.
            let clean = blocking(move || photo_file::clean(&photo))
                .await?
                .map_err(photo_error)?;
            let id = new_id();
            let capture = NewCapture {
                id,
                user_id: user.user_id,
                settings,
                exif,
                storage_key: format!("photos/{id}.{}", clean.info.format.extension()),
                photo_retention: PHOTO_RETENTION,
            };
            self.parts
                .storage
                .put(NewObject {
                    key: capture.storage_key.clone(),
                    bytes: clean.bytes,
                    content_type: clean.info.format.media_type(),
                })
                .await?;
            match self.insert(&capture).await {
                Ok(stored) => Ok(CreatedCapture {
                    capture: stored,
                    photo: clean.info,
                }),
                Err(error) => {
                    // No row points at the photo, so the purge would never find it.
                    if let Err(cleanup) = self.parts.storage.delete(&capture.storage_key).await {
                        tracing::warn!(
                            key = %capture.storage_key,
                            error = %cleanup,
                            "cannot delete the photo of a capture that wasn't saved"
                        );
                    }
                    Err(error)
                }
            }
        })
    }

    fn get<'a>(
        &'a self,
        (user, id): (&'a AuthUser, Uuid),
    ) -> BoxFuture<'a, Result<CaptureView, AppError>> {
        Box::pin(async move {
            let mut conn = self.parts.db.connection().await?;
            let capture = self.parts.captures.find(&mut conn, id).await?;
            self.view(owned((capture, user))?)
        })
    }

    fn correct<'a>(
        &'a self,
        request: CorrectionRequest<'a>,
    ) -> BoxFuture<'a, Result<(CaptureView, Corrected), AppError>> {
        Box::pin(async move {
            let CorrectionRequest {
                user,
                capture_id,
                correction,
            } = request;
            if matches!(correction, Correction::Tap(_)) {
                self.spend_run(user)?;
            }
            let mut tx = self.parts.db.begin().await?;
            let mut capture = self.lock_own((&mut tx, user, capture_id)).await?;
            let corrected = self.apply((&mut tx, &mut capture, correction)).await?;
            self.parts.captures.save(&mut tx, &capture).await?;
            tx.commit().await?;
            Ok((self.view(capture)?, corrected))
        })
    }

    fn run_vision(&self, run: RunVisionPayload) -> BoxFuture<'_, Result<(), AppError>> {
        Box::pin(async move {
            let capture = {
                let mut conn = self.parts.db.connection().await?;
                self.parts.captures.find(&mut conn, run.capture_id).await?
            };
            let Some(capture) = capture else {
                return Ok(());
            };
            if capture.run != run.run || capture.status != CaptureStatus::Pending {
                return Ok(());
            }
            let photo = match &capture.storage_key {
                Some(key) => self.parts.storage.get(key).await?,
                None => None,
            };
            let Some(photo) = photo else {
                let gone = AppError::new(ErrorCode::CapturePhotoDeleted);
                return self.finish_run((run, RunOutcome::Failed(gone))).await;
            };
            let input = vision_input(&capture.settings);
            let outcome = match self.measure((input, photo)).await? {
                Ok(result) => RunOutcome::Measured(Box::new(result)),
                Err(error) => {
                    if !error.is_users_to_fix() {
                        tracing::error!(
                            capture_id = %run.capture_id,
                            run = run.run,
                            %error,
                            "vision failed on our side"
                        );
                    }
                    RunOutcome::Failed(vision_error(&error))
                }
            };
            self.finish_run((run, outcome)).await
        })
    }

    fn give_up_run(&self, run: RunVisionPayload) -> BoxFuture<'_, Result<(), AppError>> {
        Box::pin(async move {
            let failed = AppError::new(ErrorCode::CaptureProcessingFailed);
            self.finish_run((run, RunOutcome::Failed(failed))).await
        })
    }
}

/// The capture when it belongs to `user`; `CAPTURE_NOT_FOUND` otherwise, so nobody learns
/// whether someone else's exists.
fn owned((capture, user): (Option<Capture>, &AuthUser)) -> Result<Capture, AppError> {
    match capture {
        Some(capture) if capture.user_id == user.user_id => Ok(capture),
        _ => Err(AppError::new(ErrorCode::CaptureNotFound)),
    }
}

/// Moves the capture on to a new pending run; its last outline stays until the run lands.
fn start_run(capture: &mut Capture) {
    capture.run += 1;
    capture.status = CaptureStatus::Pending;
    capture.error = None;
}

/// The capture's stored outline, read back.
fn stored_result(capture: &Capture) -> Result<Option<VisionResult>, AppError> {
    capture
        .result
        .clone()
        .map(serde_json::from_value)
        .transpose()
        .map_err(internal)
}

/// Puts `result` on the capture as its outline: done, with its confidence and level.
fn save_result((capture, result): (&mut Capture, &VisionResult)) -> Result<(), AppError> {
    capture.result = Some(to_json(result)?);
    capture.confidence = result
        .products
        .iter()
        .map(|product| product.confidence)
        .reduce(f64::min);
    capture.level_used = Some(match result.level_used {
        Level::L1 => VisionLevel::L1,
    });
    capture.status = CaptureStatus::Done;
    capture.error = None;
    Ok(())
}

fn to_json(result: &VisionResult) -> Result<serde_json::Value, AppError> {
    serde_json::to_value(result).map_err(internal)
}

/// What vision is asked for, as the engine takes it.
fn vision_input(settings: &CaptureSettings) -> vision::CaptureInput {
    vision::CaptureInput {
        paper: match settings.paper {
            Paper::A4 => PaperSize::A4,
            Paper::Letter => PaperSize::Letter,
        },
        mode: match settings.mode {
            CaptureMode::Photo => vision::CaptureMode::Photo,
            CaptureMode::Trace => vision::CaptureMode::Trace,
        },
        pen_width_mm: settings.pen_width_mm,
        tap_point_px: settings.tap_point_px,
        height_mm: settings.height_mm,
        focal_length_35mm: settings.focal_length_35mm,
    }
}

/// Checks the settings against vision's own ranges, so the API and the engine can't
/// disagree. `CAPTURE_INVALID_INPUT` names the setting.
fn check_settings(settings: &CaptureSettings) -> Result<(), AppError> {
    vision_input(settings)
        .validate()
        .map_err(|error| vision_error(&error))
}

/// Vision's error with the values its message names. The focal length comes in the upload's
/// `exif` object, so it is named as there.
fn vision_error(error: &VisionError) -> AppError {
    let code = ErrorCode::from_code(error.code()).unwrap_or(ErrorCode::CaptureProcessingFailed);
    let app_error = AppError::new(code);
    match error {
        VisionError::InvalidInput { field, allowed, .. } => {
            let field = match *field {
                "focal_length_35mm" => "exif.focal_length_35mm",
                other => other,
            };
            app_error
                .with_meta("field", field)
                .with_meta("allowed", *allowed)
        }
        VisionError::ImageTooLarge {
            width,
            height,
            max_pixels,
        } => photo_error(PhotoError::ImageTooLarge {
            width: *width,
            height: *height,
            max_pixels: *max_pixels,
        }),
        VisionError::ImageTooSmall {
            width,
            height,
            min_long_side,
        } => photo_error(PhotoError::ImageTooSmall {
            width: *width,
            height: *height,
            min_long_side: *min_long_side,
        }),
        _ => app_error,
    }
}

/// `CAPTURE_FILE_TOO_LARGE`, naming the limit.
pub fn file_too_large() -> AppError {
    AppError::new(ErrorCode::CaptureFileTooLarge).with_meta("max_mb", MAX_PHOTO_MB)
}

/// The photo check's refusal, with the numbers its message names.
fn photo_error(error: PhotoError) -> AppError {
    match error {
        PhotoError::UnsupportedImage => AppError::new(ErrorCode::CaptureUnsupportedImage),
        PhotoError::UnreadableImage => AppError::new(ErrorCode::CaptureUnreadableImage),
        PhotoError::ImageTooLarge {
            width,
            height,
            max_pixels,
        } => AppError::new(ErrorCode::CaptureImageTooLarge)
            .with_meta("width", width)
            .with_meta("height", height)
            .with_meta("max_megapixels", max_pixels / 1_000_000),
        PhotoError::ImageTooSmall {
            width,
            height,
            min_long_side,
        } => AppError::new(ErrorCode::CaptureImageTooSmall)
            .with_meta("width", width)
            .with_meta("height", height)
            .with_meta("min_long_side", min_long_side),
    }
}

/// Runs `work` off the async threads.
async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> T + Send + 'static,
) -> Result<T, AppError> {
    tokio::task::spawn_blocking(work).await.map_err(internal)
}

fn internal(error: impl std::error::Error + Send + Sync + 'static) -> AppError {
    AppError::new(ErrorCode::Internal).with_source(error)
}
