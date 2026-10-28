//! The vision output, shaped like CLAUDE.md §5.5.

use std::str::FromStr;

use serde::Serialize;

use crate::paper::PaperSize;

/// How the photo was taken (CLAUDE.md §3 step 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CaptureMode {
    /// Product photographed on the paper.
    Photo,
    /// Pen outline photographed after removing the product (no parallax).
    Trace,
}

impl FromStr for CaptureMode {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text.to_ascii_lowercase().as_str() {
            "photo" => Ok(Self::Photo),
            "trace" => Ok(Self::Trace),
            other => Err(format!("unknown mode `{other}` (use photo or trace)")),
        }
    }
}

/// Outline class used to pick the insert hole shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Shape {
    /// Circle-like: jars, round bottles, tubes seen from above.
    Round,
    /// Fills its rotated bounding box: boxes, cases.
    Rectangular,
    /// Anything else.
    Freeform,
}

/// Where the height came from. Photos can't see height yet (DEC-09: the user types it).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HeightSource {
    /// Typed by the user.
    User,
}

/// Which level of the pipeline produced the result (CLAUDE.md §5.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Level {
    /// Classic OpenCV.
    L1,
}

/// Full result for one photo.
#[derive(Debug, Clone, Serialize)]
pub struct VisionResult {
    pub mode: CaptureMode,
    pub paper: PaperResult,
    pub products: Vec<ProductResult>,
    pub warnings: Vec<String>,
    pub level_used: Level,
}

/// The sheet that was found.
#[derive(Debug, Clone, Serialize)]
pub struct PaperResult {
    pub size: PaperSize,
    /// Corners in original photo pixels: top-left, top-right, bottom-right, bottom-left.
    pub corners_px: [[f64; 2]; 4],
    pub px_per_mm: f64,
}

/// Length × width of the product's rotated bounding box.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct BboxMm {
    pub l: f64,
    pub w: f64,
}

/// One measured product.
#[derive(Debug, Clone, Serialize)]
pub struct ProductResult {
    pub id: String,
    pub shape: Shape,
    /// 0–1, see `measure::confidence`.
    pub confidence: f64,
    pub bbox_mm: BboxMm,
    /// Smallest enclosing circle, for round products only.
    pub diameter_mm: Option<f64>,
    /// Rotation of the bounding box on the paper, in degrees.
    pub angle_deg: f64,
    /// Centre on the flattened paper, in mm from its top-left corner.
    pub center_mm: [f64; 2],
    pub area_mm2: f64,
    /// Simplified outline (ε ≈ 0.2 mm), in mm from the outline's top-left corner.
    pub contour_mm: Vec<[f64; 2]>,
    pub height_mm: Option<f64>,
    pub height_source: HeightSource,
}

/// Rounds to 2 decimals for output (0.01 mm is far below the measuring precision).
pub fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}
