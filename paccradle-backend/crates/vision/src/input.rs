//! What a capture job asks the pipeline for, as stored with the job (M1-D5).

use serde::{Deserialize, Serialize};

use crate::AnalyzeOptions;
use crate::error::VisionError;
use crate::paper::PaperSize;
use crate::result::CaptureMode;

/// Narrowest and widest pen line trace mode accepts, in mm: fine liners to thin markers.
pub const PEN_WIDTH_RANGE_MM: (f64, f64) = (0.1, 3.0);
const PEN_WIDTH_ALLOWED: &str = "0.1 to 3 mm";
/// What a tap point must be (also checked in `analyze`).
pub(crate) const TAP_ALLOWED: &str = "a point in the photo, in pixels";
/// Lowest and tallest product height accepted, in mm: a card to a tall bottle and more.
pub const HEIGHT_RANGE_MM: (f64, f64) = (0.1, 1000.0);
const HEIGHT_ALLOWED: &str = "0.1 to 1000 mm";
/// Focal lengths accepted, in 35 mm terms: phone ultra-wides to long digital zoom.
pub const FOCAL_LENGTH_RANGE_MM: (f64, f64) = (5.0, 3000.0);
const FOCAL_LENGTH_ALLOWED: &str = "5 to 3000 mm";

/// One capture to measure: the paper under the product and how the photo was taken.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CaptureInput {
    pub paper: PaperSize,
    pub mode: CaptureMode,
    /// Trace mode only: the pen line's width, in mm. Half of it is added back round the
    /// traced outline (CLAUDE.md §5.2 fix A).
    #[serde(default = "default_pen_width_mm")]
    pub pen_width_mm: f64,
    /// Where the user tapped the product, in photo pixels (the frame of `corners_px`), when
    /// detection picked the wrong thing or nothing (CLAUDE.md §3 step 4, M1-C3). Detection
    /// runs again and keeps the outline under the tap. Absent on a first run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tap_point_px: Option<[f64; 2]>,
    /// The product's height in mm, typed by the user (DEC-09, M1-C4), when it's known before
    /// a run: a re-run after a tap keeps it this way. [`set_height`](crate::set_height) adds
    /// it to a stored result.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height_mm: Option<f64>,
    /// The lens's focal length in 35 mm terms, in mm, from the photo's EXIF
    /// (`FocalLengthIn35mmFilm`), which the browser reads before re-encoding the photo
    /// (M1-E2). It places the camera for the tall-product warning; without it a wide phone
    /// lens is assumed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focal_length_35mm: Option<f64>,
}

fn default_pen_width_mm() -> f64 {
    crate::DEFAULT_PEN_WIDTH_MM
}

impl CaptureInput {
    /// Checks every setting, naming the one that's wrong.
    pub fn validate(&self) -> Result<(), VisionError> {
        Bounded {
            field: "pen_width_mm",
            value: self.pen_width_mm,
            range: PEN_WIDTH_RANGE_MM,
            allowed: PEN_WIDTH_ALLOWED,
        }
        .check()?;
        if let Some(height_mm) = self.height_mm {
            check_height(height_mm)?;
        }
        if let Some(focal) = self.focal_length_35mm {
            Bounded {
                field: "focal_length_35mm",
                value: focal,
                range: FOCAL_LENGTH_RANGE_MM,
                allowed: FOCAL_LENGTH_ALLOWED,
            }
            .check()?;
        }
        // Whether it lies in the photo is checked once the photo is decoded.
        if let Some(tap) = self.tap_point_px
            && !tap.iter().all(|value| value.is_finite())
        {
            return Err(VisionError::InvalidInput {
                field: "tap_point_px",
                value: format!("{tap:?}"),
                allowed: TAP_ALLOWED,
            });
        }
        Ok(())
    }

    /// The pipeline settings for this capture: GrabCut on, no debug images.
    pub fn options(&self) -> AnalyzeOptions {
        AnalyzeOptions {
            paper: self.paper,
            mode: self.mode,
            pen_width_mm: self.pen_width_mm,
            tap_point_px: self.tap_point_px,
            height_mm: self.height_mm,
            focal_length_35mm: self.focal_length_35mm,
            ..AnalyzeOptions::default()
        }
    }
}

/// Checks a typed product height.
pub(crate) fn check_height(height_mm: f64) -> Result<(), VisionError> {
    Bounded {
        field: "height_mm",
        value: height_mm,
        range: HEIGHT_RANGE_MM,
        allowed: HEIGHT_ALLOWED,
    }
    .check()
}

/// A number setting and the range it must lie in, ends included.
struct Bounded {
    field: &'static str,
    value: f64,
    range: (f64, f64),
    allowed: &'static str,
}

impl Bounded {
    fn check(self) -> Result<(), VisionError> {
        let (min, max) = self.range;
        if (min..=max).contains(&self.value) {
            return Ok(());
        }
        Err(VisionError::InvalidInput {
            field: self.field,
            value: self.value.to_string(),
            allowed: self.allowed,
        })
    }
}
