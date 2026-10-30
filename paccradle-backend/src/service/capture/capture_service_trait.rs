use std::fmt;

use uuid::Uuid;

use crate::error::AppError;
use crate::models::capture::{
    CameraExif, CaptureSettings, CaptureView, Corrected, Correction, CreatedCapture,
    RunVisionPayload,
};
use crate::models::session::AuthUser;
use crate::shared::future::BoxFuture;

/// A photo sent to make a capture, with its settings read by the handler.
pub struct PhotoUpload {
    /// Who sends it (a guest too).
    pub user: AuthUser,
    /// What vision is asked for.
    pub settings: CaptureSettings,
    /// What the browser read from the photo's EXIF.
    pub exif: CameraExif,
    /// The file as uploaded.
    pub photo: Vec<u8>,
}

// The photo is the user's: logs show its size only.
impl fmt::Debug for PhotoUpload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PhotoUpload")
            .field("user", &self.user)
            .field("settings", &self.settings)
            .field("photo_bytes", &self.photo.len())
            .finish_non_exhaustive()
    }
}

/// A correction to one of the caller's captures.
#[derive(Debug, Clone)]
pub struct CorrectionRequest<'a> {
    /// Who asks.
    pub user: &'a AuthUser,
    /// Which capture.
    pub capture_id: Uuid,
    /// What changes.
    pub correction: Correction,
}

/// Captures: a photo on A4 or Letter paper, and what vision measures in it (CLAUDE.md §5).
pub trait CaptureService: Send + Sync {
    /// Checks the settings (vision's ranges) and the photo from its header (JPEG, PNG or
    /// WebP; at most 40 megapixels, at least 1000 px on the long side), keeps it without its
    /// metadata (only the picture, its colour profile and orientation stay), stores it for 30
    /// days, and adds a pending capture with its first vision run queued, in one transaction.
    /// `RATE_LIMITED` after `RATE_LIMIT_UPLOADS_PER_HOUR` vision runs (uploads and taps) in
    /// an hour.
    fn create(&self, upload: PhotoUpload) -> BoxFuture<'_, Result<CreatedCapture, AppError>>;

    /// The caller's capture, with a fresh 10-minute link to its photo while it is kept.
    /// `CAPTURE_NOT_FOUND` for anyone else's.
    fn get<'a>(
        &'a self,
        request: (&'a AuthUser, Uuid),
    ) -> BoxFuture<'a, Result<CaptureView, AppError>>;

    /// Applies a correction (CLAUDE.md §3 steps 4–5):
    ///
    /// - a **tap** queues a new run keeping the outline under it (`Running`); the last
    ///   outline stays until the run lands. `CAPTURE_PHOTO_DELETED` once the photo is gone;
    ///   it counts against the hourly vision runs.
    /// - an **edit** is measured as drawn, at once (`Done`), and outdates any pending run.
    ///   `CAPTURE_NOT_READY` without an outline to edit; `CAPTURE_OUTLINE_CROSSES_ITSELF`.
    /// - a **height** is kept for every later run and applied to the outline at once
    ///   (`Done`); while a run is pending, a new run with it is queued instead (`Running`).
    fn correct<'a>(
        &'a self,
        request: CorrectionRequest<'a>,
    ) -> BoxFuture<'a, Result<(CaptureView, Corrected), AppError>>;

    /// One vision run (the `run_vision` job): reads the photo, measures it, and saves the
    /// result or the error, unless the run is no longer the capture's. Storage or database
    /// outages are errors for the job to retry; everything vision says is saved.
    fn run_vision(&self, run: RunVisionPayload) -> BoxFuture<'_, Result<(), AppError>>;

    /// Marks a run whose job failed for good as failed (`CAPTURE_PROCESSING_FAILED`), if it
    /// is still the capture's pending run.
    fn give_up_run(&self, run: RunVisionPayload) -> BoxFuture<'_, Result<(), AppError>>;
}
