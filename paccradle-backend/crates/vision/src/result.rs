//! The vision output, shaped like CLAUDE.md §5.5. The API stores it with the capture and
//! reads it back, so every type reads from JSON as well as writing to it.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::paper::PaperSize;

/// How the photo was taken (CLAUDE.md §3 step 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HeightSource {
    /// Typed by the user.
    User,
}

/// Which level of the pipeline produced the result (CLAUDE.md §5.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Level {
    /// Classic OpenCV.
    L1,
}

/// Full result for one photo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VisionResult {
    pub mode: CaptureMode,
    pub paper: PaperResult,
    pub products: Vec<ProductResult>,
    pub warnings: Vec<PhotoWarning>,
    pub level_used: Level,
}

/// Something to tell the user about the photo (CLAUDE.md §5.5 `warnings`). The API and
/// the editor translate it by `kind`; `Display` is the English fallback.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PhotoWarning {
    /// Photo mode: from above, a tall product's top looks bigger than it is (§5.2).
    TallProductLooksBigger,
    /// The product touches the paper's edge.
    TouchesPaperEdge,
    /// Other objects lie on the paper.
    OtherObjects { count: usize },
    /// The outline may be off.
    LowConfidence { confidence: f64 },
}

impl fmt::Display for PhotoWarning {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PhotoWarning::TallProductLooksBigger => formatter.write_str(
                "From above, a tall product's top looks bigger than it is. If it is taller than \
                 about 20 mm, enter its height or use Trace mode for an exact fit.",
            ),
            PhotoWarning::TouchesPaperEdge => formatter.write_str(
                "The product touches the paper edge: move it to the middle, at least 2 cm from \
                 the edges.",
            ),
            PhotoWarning::OtherObjects { count } => write!(
                formatter,
                "Found {count} other object(s) on the paper: remove them, or tap the product."
            ),
            PhotoWarning::LowConfidence { .. } => formatter.write_str(
                "Low confidence: check the outline, or retake the photo in softer light.",
            ),
        }
    }
}

/// The sheet that was found.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaperResult {
    pub size: PaperSize,
    /// Corners in original photo pixels: top-left, top-right, bottom-right, bottom-left.
    pub corners_px: [[f64; 2]; 4],
    pub px_per_mm: f64,
}

/// Length × width of the product's rotated bounding box.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BboxMm {
    pub l: f64,
    pub w: f64,
}

/// One measured product.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
