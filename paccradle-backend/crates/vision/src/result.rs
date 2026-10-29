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

/// Where the height came from. Photos can't see height yet (DEC-09: the user types it,
/// M1-C4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HeightSource {
    /// Typed by the user.
    User,
}

/// Where a product's outline came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContourSource {
    /// Found in the photo, at the user's tap if they gave one.
    Detected,
    /// Edited by the user and measured as drawn (M1-C3).
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
    /// Where the camera was, from the paper's corners and the lens; `None` when the corners
    /// don't give a believable camera.
    pub camera: Option<CameraEstimate>,
    pub products: Vec<ProductResult>,
    pub warnings: Vec<PhotoWarning>,
    pub level_used: Level,
}

/// Where the camera was when the photo was taken (M1-C4). A tall product's top looks
/// bigger the nearer it is to the camera (CLAUDE.md §5.2), so the tall-product warning
/// needs both numbers.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CameraEstimate {
    /// Height of the lens above the paper, in mm.
    pub height_mm: f64,
    /// The point on the paper's plane straight below the lens, in mm from the paper's
    /// top-left corner. It lies off the sheet when the phone was tilted a lot.
    pub foot_mm: [f64; 2],
    pub focal_source: FocalSource,
}

/// Where the lens's focal length came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FocalSource {
    /// The photo's EXIF, sent by the browser.
    Exif,
    /// Not known: a wide phone lens is assumed (`camera::ASSUMED_FOCAL_LENGTH_35MM`).
    Assumed,
}

/// Something to tell the user about the photo (CLAUDE.md §5.5 `warnings`). The API and
/// the editor translate it by `kind`; `Display` is the English fallback.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PhotoWarning {
    /// Photo mode, height not known: from above, a tall product's top looks bigger than it
    /// is (§5.2).
    TallProductLooksBigger,
    /// Photo mode, height known: the outline may reach up to `up_to_mm` past the product's
    /// real edge, on the side away from the camera (M1-C4).
    OutlineMayBeBigger { height_mm: f64, up_to_mm: f64 },
    /// The product touches the paper's edge.
    TouchesPaperEdge,
    /// Other objects lie on the paper.
    OtherObjects { count: usize },
    /// The outline may be off.
    LowConfidence { confidence: f64 },
}

impl PhotoWarning {
    /// True for the warnings about a tall product's top looking bigger.
    pub fn is_parallax(&self) -> bool {
        matches!(
            self,
            PhotoWarning::TallProductLooksBigger | PhotoWarning::OutlineMayBeBigger { .. }
        )
    }
}

impl fmt::Display for PhotoWarning {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PhotoWarning::TallProductLooksBigger => formatter.write_str(
                "From above, a tall product's top looks bigger than it is. Use Trace mode for an \
                 exact fit, or enter its height to see by how much.",
            ),
            PhotoWarning::OutlineMayBeBigger {
                height_mm,
                up_to_mm,
            } => write!(
                formatter,
                "At {height_mm} mm tall, the product's top looks bigger from above: its outline \
                 may reach up to {up_to_mm} mm past the real edge. Use Trace mode for an exact \
                 fit, or take the photo from higher up, straight above the product."
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
    /// 0–1, see `measure::confidence`; 1 for an outline the user edited.
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
    /// The same points in photo pixels (the frame of `corners_px`), lying over the product
    /// in the photo: the editor draws them and lets the user drag them (M1-E3). An edited
    /// outline comes back in this frame.
    pub contour_px: Vec<[f64; 2]>,
    pub contour_source: ContourSource,
    /// Typed by the user (DEC-09), once they have.
    pub height_mm: Option<f64>,
    pub height_source: HeightSource,
}

/// Rounds to 2 decimals for output (0.01 mm is far below the measuring precision).
pub fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}
