//! Paccradle vision pipeline (CLAUDE.md §5).
//!
//! Level 1 (OpenCV): find the A4/Letter paper, flatten it to 10 px/mm, cut out the
//! product (photo mode) or the pen outline (trace mode), measure it in millimetres and
//! score the confidence. Built as the accuracy spike (M0-08 to M0-10), made the
//! production crate in M1-C1.
//!
//! A capture job (M1-D5) calls [`run`]: the stored [`CaptureInput`] and the uploaded
//! file's bytes in, the §5.5 [`VisionResult`] out, or a [`VisionError`] with a stable
//! [`code`](VisionError::code). It's plain computation with no I/O, and it blocks for a
//! second or so: call it from a blocking task, not on the async runtime.
//! [`image::inspect`] checks an upload's format and size without decoding it.

pub mod error;
pub mod image;
pub mod input;
pub mod measure;
pub mod paper;
pub mod result;
pub mod segment;

use opencv::core::{Mat, Point, Scalar, Vector};
use opencv::imgproc;
use opencv::prelude::*;

pub use error::VisionError;
pub use image::{ImageFormat, ImageInfo, MAX_PIXELS, MIN_LONG_SIDE_PX, decode, inspect};
pub use input::{CaptureInput, PEN_WIDTH_RANGE_MM};
pub use paper::{PX_PER_MM, PaperSize};
pub use result::{CaptureMode, PhotoWarning, Shape, VisionResult};

use measure::Measurement;
use result::{BboxMm, HeightSource, Level, PaperResult, ProductResult, round2};

/// Default pen width for trace mode (CLAUDE.md §5.2 fix A).
pub const DEFAULT_PEN_WIDTH_MM: f64 = 0.5;
/// Below this confidence the result gets a "check the outline" warning.
const LOW_CONFIDENCE: f64 = 0.6;
/// Debug drawing colours (BGR) and line widths.
const QUAD_COLOR: (f64, f64, f64) = (0.0, 200.0, 0.0);
const OUTLINE_COLOR: (f64, f64, f64) = (0.0, 0.0, 255.0);
const DEBUG_LINE_PX: i32 = 3;

/// Settings for one analysis.
#[derive(Debug, Clone)]
pub struct AnalyzeOptions {
    pub paper: PaperSize,
    pub mode: CaptureMode,
    /// Trace mode only: width of the pen line in mm.
    pub pen_width_mm: f64,
    /// Photo mode only: refine the first mask with GrabCut.
    pub refine_with_grabcut: bool,
    /// Keep intermediate images for debugging.
    pub collect_debug: bool,
}

impl Default for AnalyzeOptions {
    fn default() -> Self {
        Self {
            paper: PaperSize::A4,
            mode: CaptureMode::Photo,
            pen_width_mm: DEFAULT_PEN_WIDTH_MM,
            refine_with_grabcut: true,
            collect_debug: false,
        }
    }
}

/// Intermediate images, for checking what the pipeline saw.
pub struct DebugImages {
    /// Original photo with the found paper outlined.
    pub paper: Mat,
    /// Top-down paper at 10 px/mm.
    pub flattened: Mat,
    /// Product mask (white = product).
    pub mask: Mat,
    /// Flattened paper with the measured outline drawn on it.
    pub outline: Mat,
}

/// The result plus optional debug images.
pub struct Analysis {
    pub result: VisionResult,
    pub debug: Option<DebugImages>,
}

/// Measures one capture: checks `input`, decodes the photo (refusing files that aren't
/// JPEG, PNG or WebP, or have too many or too few pixels) and runs the Level 1 pipeline.
pub fn run((input, image): (&CaptureInput, &[u8])) -> Result<VisionResult, VisionError> {
    input.validate()?;
    let photo = decode(image)?;
    Ok(analyze(&photo, &input.options())?.result)
}

/// Runs the Level 1 pipeline on one photo (BGR, EXIF orientation already applied).
pub fn analyze(image: &Mat, options: &AnalyzeOptions) -> Result<Analysis, VisionError> {
    let quad = paper::find_paper(image)?;
    let flat = paper::flatten(image, &quad, options.paper)?;
    let segmented = match options.mode {
        CaptureMode::Photo => segment::product_mask(&flat, options.refine_with_grabcut)?,
        CaptureMode::Trace => segment::trace_mask(&flat, options.pen_width_mm)?,
    };
    let measured = measure::measure(&segmented.mask, &flat)?;
    let confidence = measure::confidence(&measured, segmented.extra_blobs);

    let result = VisionResult {
        mode: options.mode,
        paper: PaperResult {
            size: options.paper,
            corners_px: quad
                .corners
                .map(|p| [round2(f64::from(p.x)), round2(f64::from(p.y))]),
            px_per_mm: PX_PER_MM,
        },
        warnings: warnings(&WarningInputs {
            mode: options.mode,
            measured: &measured,
            extra_blobs: segmented.extra_blobs,
            confidence,
        }),
        products: vec![product_result(&measured, confidence)],
        level_used: Level::L1,
    };

    let debug = if options.collect_debug {
        Some(DebugImages {
            paper: draw_closed(image, &quad.corners.map(|p| (p.x, p.y)), QUAD_COLOR)?,
            outline: draw_outline(&flat, &segmented.mask)?,
            flattened: flat,
            mask: segmented.mask,
        })
    } else {
        None
    };
    Ok(Analysis { result, debug })
}

fn product_result(measured: &Measurement, confidence: f64) -> ProductResult {
    ProductResult {
        id: "p1".to_owned(),
        shape: measured.shape,
        confidence: round2(confidence),
        bbox_mm: BboxMm {
            l: round2(measured.length_mm),
            w: round2(measured.width_mm),
        },
        diameter_mm: measured.diameter_mm.map(round2),
        angle_deg: round2(measured.angle_deg),
        center_mm: measured.center_mm.map(round2),
        area_mm2: round2(measured.area_mm2),
        contour_mm: measured
            .contour_mm
            .iter()
            .map(|point| point.map(round2))
            .collect(),
        height_mm: None,
        height_source: HeightSource::User,
    }
}

struct WarningInputs<'a> {
    mode: CaptureMode,
    measured: &'a Measurement,
    extra_blobs: usize,
    confidence: f64,
}

/// Hints about the photo (CLAUDE.md §5.5 `warnings`).
fn warnings(inputs: &WarningInputs) -> Vec<PhotoWarning> {
    let mut list = Vec::new();
    if inputs.mode == CaptureMode::Photo {
        list.push(PhotoWarning::TallProductLooksBigger);
    }
    if inputs.measured.touches_edge {
        list.push(PhotoWarning::TouchesPaperEdge);
    }
    if inputs.extra_blobs > 0 {
        list.push(PhotoWarning::OtherObjects {
            count: inputs.extra_blobs,
        });
    }
    if inputs.confidence < LOW_CONFIDENCE {
        list.push(PhotoWarning::LowConfidence {
            confidence: round2(inputs.confidence),
        });
    }
    list
}

fn draw_closed(
    image: &Mat,
    points: &[(f32, f32)],
    color: (f64, f64, f64),
) -> Result<Mat, VisionError> {
    let mut canvas = image.try_clone()?;
    let polygon = Vector::<Vector<Point>>::from_iter([Vector::from_iter(
        points
            .iter()
            .map(|(x, y)| Point::new(x.round() as i32, y.round() as i32)),
    )]);
    let line_px = DEBUG_LINE_PX * (image.cols() / 1000).max(1);
    imgproc::polylines(
        &mut canvas,
        &polygon,
        true,
        Scalar::new(color.0, color.1, color.2, 0.0),
        line_px,
        imgproc::LINE_AA,
        0,
    )?;
    Ok(canvas)
}

fn draw_outline(flat: &Mat, mask: &Mat) -> Result<Mat, VisionError> {
    let mut canvas = flat.try_clone()?;
    let mut contours = Vector::<Vector<Point>>::new();
    imgproc::find_contours_def(
        mask,
        &mut contours,
        imgproc::RETR_EXTERNAL,
        imgproc::CHAIN_APPROX_SIMPLE,
    )?;
    imgproc::draw_contours(
        &mut canvas,
        &contours,
        -1,
        Scalar::new(OUTLINE_COLOR.0, OUTLINE_COLOR.1, OUTLINE_COLOR.2, 0.0),
        DEBUG_LINE_PX,
        imgproc::LINE_AA,
        &opencv::core::no_array(),
        i32::MAX,
        Point::default(),
    )?;
    Ok(canvas)
}
