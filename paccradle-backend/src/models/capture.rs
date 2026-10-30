//! Photos and what vision measured in them (the `captures` table, CLAUDE.md §5, §8.4).

use std::time::Duration;

use photo_file::ImageInfo;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use time::OffsetDateTime;
use uuid::Uuid;

use super::TextEnum;
use super::job::JobKind;
use super::paper::Paper;

/// How long a photo is kept before the purge deletes it (CLAUDE.md §8.5).
pub const PHOTO_RETENTION: Duration = Duration::from_secs(30 * 24 * 60 * 60);
/// Trace mode's pen width when none is given, in mm. The ranges every setting must lie in
/// are vision's own (`vision::CaptureInput::validate`, checked by the capture service).
pub const DEFAULT_PEN_WIDTH_MM: f64 = vision::DEFAULT_PEN_WIDTH_MM;

/// `captures.status`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureStatus {
    /// Waiting for vision.
    Pending,
    /// Measured: the result is stored.
    Done,
    /// Vision couldn't measure it: the error code is stored.
    Failed,
}

impl TextEnum for CaptureStatus {
    const ALL: &'static [Self] = &[Self::Pending, Self::Done, Self::Failed];

    fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Done => "done",
            Self::Failed => "failed",
        }
    }
}

/// `captures.mode`: how the photo was taken (CLAUDE.md §3 step 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureMode {
    /// The product on the paper.
    Photo,
    /// The pen line drawn round the product, after taking it away (no parallax).
    Trace,
}

impl TextEnum for CaptureMode {
    const ALL: &'static [Self] = &[Self::Photo, Self::Trace];

    fn as_str(self) -> &'static str {
        match self {
            Self::Photo => "photo",
            Self::Trace => "trace",
        }
    }
}

/// What vision is asked to measure (`captures.input`), in the JSON shape of
/// `vision::CaptureInput`, which the vision job reads back (M1-D5).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CaptureSettings {
    /// The sheet under the product.
    pub paper: Paper,
    /// Photo or trace.
    pub mode: CaptureMode,
    /// Trace mode: the pen line's width, in mm.
    pub pen_width_mm: f64,
    /// The product's height in mm, when the user typed it before the photo.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height_mm: Option<f64>,
    /// The lens's focal length in 35 mm terms, from the photo's EXIF.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focal_length_35mm: Option<f64>,
    /// Where the user last tapped the product, in photo pixels (M1-D5): runs after a tap
    /// keep the outline under it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tap_point_px: Option<[f64; 2]>,
}

/// `captures.level_used`: which level of the vision pipeline measured it (CLAUDE.md §5.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisionLevel {
    /// Classic OpenCV.
    L1,
    /// SAM 2 (M3).
    L2,
    /// The AI helper (M3).
    L3,
}

impl TextEnum for VisionLevel {
    const ALL: &'static [Self] = &[Self::L1, Self::L2, Self::L3];

    fn as_str(self) -> &'static str {
        match self {
            Self::L1 => "l1",
            Self::L2 => "l2",
            Self::L3 => "l3",
        }
    }
}

/// Why a capture's last run failed, as stored: the error code and the values its message
/// names.
#[derive(Debug, Clone, PartialEq)]
pub struct StoredError {
    /// E.g. `CAPTURE_PAPER_NOT_FOUND`.
    pub code: String,
    /// E.g. `{ "field": "tap_point_px", "allowed": "…" }`; often empty.
    pub meta: Map<String, Value>,
}

/// The camera fields the browser read from the photo's EXIF (`captures.exif`). Nothing else
/// is kept, never a location.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CameraExif {
    /// The phone's maker, e.g. `Apple`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub make: Option<String>,
    /// The phone's model, e.g. `iPhone 15 Pro`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// The lens's focal length in 35 mm terms, in mm.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focal_length_35mm: Option<f64>,
}

/// A capture to add, its photo already stored.
#[derive(Debug, Clone)]
pub struct NewCapture {
    /// The id it will have; the photo's key is made from it.
    pub id: Uuid,
    /// Whose it is.
    pub user_id: Uuid,
    /// What vision is asked for.
    pub settings: CaptureSettings,
    /// What the browser read from the photo's EXIF.
    pub exif: CameraExif,
    /// Where the photo is stored.
    pub storage_key: String,
    /// How long the photo is kept.
    pub photo_retention: Duration,
}

/// A capture as stored. The service changes the fields under its row lock, then saves them.
#[derive(Debug, Clone, PartialEq)]
pub struct Capture {
    /// The capture's id.
    pub id: Uuid,
    /// Whose it is.
    pub user_id: Uuid,
    /// Where vision is with its current run.
    pub status: CaptureStatus,
    /// What vision is asked for.
    pub settings: CaptureSettings,
    /// The current run's number: 1 at first, moved on by a tap, an edit, or a height typed
    /// while a run is pending. A run's job saves its result only while this is still its own.
    pub run: i32,
    /// The latest outline: the §5.5 `VisionResult` JSON. Kept while a newer run is pending
    /// or after it fails.
    pub result: Option<Value>,
    /// The result's lowest product confidence (0–1).
    pub confidence: Option<f64>,
    /// The level that made the result.
    pub level_used: Option<VisionLevel>,
    /// Why the current run failed (status `failed`).
    pub error: Option<StoredError>,
    /// Where the photo is stored; `None` once it was deleted.
    pub storage_key: Option<String>,
    /// When it was made.
    pub created_at: OffsetDateTime,
    /// When its photo is deleted, at the latest.
    pub purge_after: OffsetDateTime,
}

/// A capture just made, with what its photo's header said.
#[derive(Debug, Clone, PartialEq)]
pub struct CreatedCapture {
    /// The capture.
    pub capture: Capture,
    /// The stored photo's format and size.
    pub photo: ImageInfo,
}

/// A capture, with a link to its photo while it is kept.
#[derive(Debug, Clone, PartialEq)]
pub struct CaptureView {
    /// The capture.
    pub capture: Capture,
    /// A signed URL to the photo and when it stops working; `None` once the photo is deleted.
    pub photo_url: Option<(String, OffsetDateTime)>,
}

/// A change the user makes to a capture (CLAUDE.md §3 steps 4–5).
#[derive(Debug, Clone, PartialEq)]
pub enum Correction {
    /// "The product is here", in photo pixels: measure again, keeping the outline under it.
    Tap([f64; 2]),
    /// The outline as the user drew it, in photo pixels: taken as it is.
    Edit(Vec<[f64; 2]>),
    /// The product's height, in mm.
    Height(f64),
}

/// What a correction did: answered at once, or started a vision run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Corrected {
    /// The capture is up to date.
    Done,
    /// A new run is pending; poll the capture.
    Running,
}

/// A stored photo past its `purge_after`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuePhoto {
    /// Its capture.
    pub capture_id: Uuid,
    /// Where it is stored.
    pub storage_key: String,
}

/// Measures a capture's photo (CLAUDE.md §5): one run of vision.
pub struct RunVision;

/// The capture and the run to make. A run that is no longer the capture's is skipped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunVisionPayload {
    /// The capture.
    pub capture_id: Uuid,
    /// Its run number when the job was queued.
    pub run: i32,
}

impl JobKind for RunVision {
    const NAME: &'static str = "run_vision";
    type Payload = RunVisionPayload;
}

/// Deletes the photos past their 30 days (the worker schedules it daily).
pub struct PurgePhotos;

/// [`PurgePhotos`] needs nothing: it looks up what is due.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct PurgePhotosPayload {}

impl JobKind for PurgePhotos {
    const NAME: &'static str = "purge_photos";
    /// Retried for about an hour and a half (10 s, doubling) before waiting for the next
    /// day's run: a short storage outage shouldn't keep photos a day longer.
    const MAX_ATTEMPTS: i32 = 10;
    type Payload = PurgePhotosPayload;
}
