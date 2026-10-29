//! What a capture job asks the pipeline for, as stored with the job (M1-D5).

use serde::{Deserialize, Serialize};

use crate::AnalyzeOptions;
use crate::error::VisionError;
use crate::paper::PaperSize;
use crate::result::CaptureMode;

/// Narrowest and widest pen line trace mode accepts, in mm: fine liners to thin markers.
pub const PEN_WIDTH_RANGE_MM: (f64, f64) = (0.1, 3.0);
const PEN_WIDTH_ALLOWED: &str = "0.1 to 3 mm";

/// One capture to measure: the paper under the product and how the photo was taken.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CaptureInput {
    pub paper: PaperSize,
    pub mode: CaptureMode,
    /// Trace mode only: the pen line's width, in mm. Half of it is added back round the
    /// traced outline (CLAUDE.md §5.2 fix A).
    #[serde(default = "default_pen_width_mm")]
    pub pen_width_mm: f64,
}

fn default_pen_width_mm() -> f64 {
    crate::DEFAULT_PEN_WIDTH_MM
}

impl CaptureInput {
    /// Checks every setting, naming the one that's wrong.
    pub fn validate(&self) -> Result<(), VisionError> {
        let (min, max) = PEN_WIDTH_RANGE_MM;
        let pen = self.pen_width_mm;
        if !pen.is_finite() || pen < min || pen > max {
            return Err(VisionError::InvalidInput {
                field: "pen_width_mm",
                value: pen.to_string(),
                allowed: PEN_WIDTH_ALLOWED,
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
            ..AnalyzeOptions::default()
        }
    }
}
