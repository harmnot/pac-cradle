//! Capture bodies: the settings sent with a photo and the answer to an upload (M1-D4); a
//! capture with its outline, and corrections (M1-D5).

use photo_file::ImageFormat;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{AppError, ErrorCode, ErrorReport};
use crate::locales::Locale;
use crate::models::capture::{
    CameraExif, CaptureMode, CaptureSettings, CaptureStatus, CaptureView, Correction,
    CreatedCapture, DEFAULT_PEN_WIDTH_MM, StoredError,
};
use crate::models::paper::Paper;

/// The longest camera make or model kept, in characters.
const MAX_CAMERA_NAME_CHARS: usize = 100;

/// The `capture` part of `POST /captures` (JSON): what vision is asked for, and what the
/// browser read from the photo's EXIF.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateCaptureRequest {
    /// `a4` or `letter`.
    pub paper: Paper,
    /// `photo` or `trace`.
    pub mode: CaptureMode,
    /// Trace mode: the pen line's width, 0.1 to 3 mm; default 0.5.
    #[serde(default)]
    pub pen_width_mm: Option<f64>,
    /// The product's height, 0.1 to 1000 mm, if the user typed it before the photo.
    #[serde(default)]
    pub height_mm: Option<f64>,
    /// What the browser read from the photo's EXIF.
    #[serde(default)]
    pub exif: ExifRequest,
}

/// EXIF fields the browser read (with `exifr`) before re-encoding the photo. Only these are
/// kept: any other field, a location included, is dropped unread.
#[derive(Debug, Default, Deserialize)]
pub struct ExifRequest {
    /// `Make`, e.g. `Apple`.
    #[serde(default)]
    pub make: Option<String>,
    /// `Model`, e.g. `iPhone 15 Pro`.
    #[serde(default)]
    pub model: Option<String>,
    /// `FocalLengthIn35mmFormat`, 5 to 3000 mm.
    #[serde(default)]
    pub focal_length_35mm: Option<f64>,
}

impl CreateCaptureRequest {
    /// Reads the part's bytes. Anything that isn't this JSON object is `INVALID_REQUEST`
    /// (`meta.field = "capture"`).
    pub fn from_json(bytes: &[u8]) -> Result<Self, AppError> {
        serde_json::from_slice(bytes).map_err(|error| {
            AppError::new(ErrorCode::InvalidRequest)
                .with_meta("field", "capture")
                .with_source(error)
        })
    }

    /// The settings for vision and the camera fields to keep. The camera names are checked
    /// here; the numbers' ranges are vision's, checked by the capture service.
    pub fn into_parts(self) -> Result<(CaptureSettings, CameraExif), AppError> {
        let focal_length_35mm = self.exif.focal_length_35mm;
        let settings = CaptureSettings {
            paper: self.paper,
            mode: self.mode,
            pen_width_mm: self.pen_width_mm.unwrap_or(DEFAULT_PEN_WIDTH_MM),
            height_mm: self.height_mm,
            focal_length_35mm,
            tap_point_px: None,
        };
        let exif = CameraExif {
            make: camera_name(("exif.make", self.exif.make))?,
            model: camera_name(("exif.model", self.exif.model))?,
            focal_length_35mm,
        };
        Ok((settings, exif))
    }
}

/// A camera make or model: trimmed, control characters dropped, empty as none, and at most
/// [`MAX_CAMERA_NAME_CHARS`] characters.
fn camera_name((field, name): (&str, Option<String>)) -> Result<Option<String>, AppError> {
    let Some(name) = name else {
        return Ok(None);
    };
    let name: String = name.chars().filter(|c| !c.is_control()).collect();
    let name = name.trim();
    if name.chars().count() > MAX_CAMERA_NAME_CHARS {
        return Err(AppError::new(ErrorCode::CaptureInvalidInput)
            .with_meta("field", field)
            .with_meta(
                "allowed",
                format!("at most {MAX_CAMERA_NAME_CHARS} characters"),
            ));
    }
    Ok((!name.is_empty()).then(|| name.to_owned()))
}

/// `POST /captures` → 201: the capture, waiting for vision.
#[derive(Debug, Serialize)]
pub struct CaptureCreatedResponse {
    /// The capture's id, for `GET /captures/{capture_id}` (M1-D5).
    pub capture_id: Uuid,
    /// `pending` until vision has run.
    pub status: CaptureStatus,
    /// As sent.
    pub mode: CaptureMode,
    /// As sent.
    pub paper: Paper,
    /// What the photo's header says.
    pub photo: PhotoResponse,
    /// When the capture was made.
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    /// When the photo is deleted, at the latest (outlines and sizes stay).
    #[serde(with = "time::serde::rfc3339")]
    pub photo_kept_until: OffsetDateTime,
}

/// A stored photo's format and size, as stored (before its EXIF turn).
#[derive(Debug, Serialize)]
pub struct PhotoResponse {
    /// `jpeg`, `png` or `webp`.
    pub format: ImageFormat,
    /// In pixels.
    pub width: u32,
    /// In pixels.
    pub height: u32,
}

impl From<CreatedCapture> for CaptureCreatedResponse {
    fn from(created: CreatedCapture) -> Self {
        let capture = created.capture;
        Self {
            capture_id: capture.id,
            status: capture.status,
            mode: capture.settings.mode,
            paper: capture.settings.paper,
            photo: PhotoResponse {
                format: created.photo.format,
                width: created.photo.width,
                height: created.photo.height,
            },
            created_at: capture.created_at,
            photo_kept_until: capture.purge_after,
        }
    }
}

/// `POST /captures/{capture_id}/correct`: exactly one of the three.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorrectCaptureRequest {
    /// "The product is here": `[x, y]` in photo pixels (the frame of `contour_px`).
    #[serde(default)]
    pub tap_point_px: Option<[f64; 2]>,
    /// The outline as drawn: `[[x, y], …]` in photo pixels, 3 to 2,000 points, in order.
    #[serde(default)]
    pub edited_contour_px: Option<Vec<[f64; 2]>>,
    /// The product's height in mm, 0.1 to 1000.
    #[serde(default)]
    pub height_mm: Option<f64>,
}

impl CorrectCaptureRequest {
    /// The correction. None or more than one is `INVALID_REQUEST`.
    pub fn correction(self) -> Result<Correction, AppError> {
        match (self.tap_point_px, self.edited_contour_px, self.height_mm) {
            (Some(tap), None, None) => Ok(Correction::Tap(tap)),
            (None, Some(outline), None) => Ok(Correction::Edit(outline)),
            (None, None, Some(height_mm)) => Ok(Correction::Height(height_mm)),
            _ => Err(AppError::new(ErrorCode::InvalidRequest)
                .with_meta("field", "correction")
                .with_meta(
                    "expected",
                    "one of tap_point_px, edited_contour_px, height_mm",
                )),
        }
    }
}

/// A capture: `GET /captures/{capture_id}`, and the answer to a correction.
#[derive(Debug, Serialize)]
pub struct CaptureResponse {
    /// The capture's id.
    pub capture_id: Uuid,
    /// `pending` while vision runs (poll again), `done`, or `failed` (see `error`).
    pub status: CaptureStatus,
    /// As sent.
    pub mode: CaptureMode,
    /// As sent.
    pub paper: Paper,
    /// What vision works with besides the paper and mode.
    pub settings: SettingsResponse,
    /// The latest outline, CLAUDE.md §5.5: paper, products (`contour_mm` and `contour_px`),
    /// warnings (codes), camera, level. Kept while a newer run is pending or after it failed;
    /// `null` until the first run succeeds.
    pub result: Option<Value>,
    /// Why the latest run failed (status `failed`), in the caller's language.
    pub error: Option<CaptureErrorResponse>,
    /// The photo, for drawing outlines over it; `null` once it was deleted.
    pub photo: Option<PhotoLinkResponse>,
    /// When the capture was made.
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    /// When the photo is deleted, at the latest.
    #[serde(with = "time::serde::rfc3339")]
    pub photo_kept_until: OffsetDateTime,
}

/// The settings vision uses besides the paper and mode.
#[derive(Debug, Serialize)]
pub struct SettingsResponse {
    /// Trace mode's pen width, in mm.
    pub pen_width_mm: f64,
    /// The product's height, once typed.
    pub height_mm: Option<f64>,
    /// The lens's focal length in 35 mm terms, from EXIF.
    pub focal_length_35mm: Option<f64>,
    /// The last tap, in photo pixels.
    pub tap_point_px: Option<[f64; 2]>,
}

/// A failed run's error: the code to branch on, its message, and the message's values.
#[derive(Debug, Serialize)]
pub struct CaptureErrorResponse {
    /// E.g. `CAPTURE_PAPER_NOT_FOUND`.
    pub code: &'static str,
    /// In the caller's language.
    pub message: String,
    /// The values in the message, e.g. `{ "field": "tap_point_px" }`.
    pub meta: Map<String, Value>,
}

/// A link to the photo that works for 10 minutes.
#[derive(Debug, Serialize)]
pub struct PhotoLinkResponse {
    /// The signed URL.
    pub url: String,
    /// When it stops working: fetch the capture again for a new one.
    #[serde(with = "time::serde::rfc3339")]
    pub url_expires_at: OffsetDateTime,
}

impl CaptureResponse {
    /// The capture as the caller sees it, its error in `locale`.
    pub fn new((view, locale): (CaptureView, Locale)) -> Self {
        let capture = view.capture;
        Self {
            capture_id: capture.id,
            status: capture.status,
            mode: capture.settings.mode,
            paper: capture.settings.paper,
            settings: SettingsResponse {
                pen_width_mm: capture.settings.pen_width_mm,
                height_mm: capture.settings.height_mm,
                focal_length_35mm: capture.settings.focal_length_35mm,
                tap_point_px: capture.settings.tap_point_px,
            },
            result: capture.result,
            error: capture.error.map(|error| error_response((error, locale))),
            photo: view
                .photo_url
                .map(|(url, url_expires_at)| PhotoLinkResponse {
                    url,
                    url_expires_at,
                }),
            created_at: capture.created_at,
            photo_kept_until: capture.purge_after,
        }
    }
}

/// A stored error in `locale`. A code this release doesn't know (stored by a newer one)
/// reads as `CAPTURE_PROCESSING_FAILED`.
fn error_response((error, locale): (StoredError, Locale)) -> CaptureErrorResponse {
    let code = ErrorCode::from_code(&error.code).unwrap_or(ErrorCode::CaptureProcessingFailed);
    let report = ErrorReport {
        code,
        meta: error.meta,
    };
    CaptureErrorResponse {
        code: code.as_str(),
        message: locale.error_message(&report),
        meta: report.meta,
    }
}
