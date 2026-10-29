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
//!
//! Manual corrections (CLAUDE.md §3 step 4, M1-C3): a tap on the product
//! ([`CaptureInput::tap_point_px`]) re-runs [`run`] and keeps the outline under the tap;
//! [`accept_edit`] measures an outline the user edited exactly as drawn, from the stored
//! result alone. Both speak photo pixels, the frame of `corners_px` and `contour_px`.
//!
//! Height (CLAUDE.md §5.2 fix B, DEC-09, M1-C4): the user types it, through
//! [`CaptureInput::height_mm`] or [`set_height`]. With the camera worked out from the paper's
//! corners and the lens ([`camera`]), the tall-product warning says how far the outline may
//! reach past the product's real edge, or goes away when that's too little to matter.

pub mod camera;
pub mod edit;
pub mod error;
pub mod height;
pub mod image;
pub mod input;
pub mod measure;
pub mod paper;
pub mod result;
pub mod segment;

use opencv::core::{Mat, Point, Point2f, Scalar, Vector};
use opencv::imgproc;
use opencv::prelude::*;

pub use edit::{MAX_EDIT_POINTS, accept_edit};
pub use error::VisionError;
pub use height::{PARALLAX_WARN_MM, set_height};
pub use image::{ImageFormat, ImageInfo, MAX_PIXELS, MIN_LONG_SIDE_PX, decode, inspect};
pub use input::{CaptureInput, FOCAL_LENGTH_RANGE_MM, HEIGHT_RANGE_MM, PEN_WIDTH_RANGE_MM};
pub use paper::{PX_PER_MM, PaperSize};
pub use result::{
    CameraEstimate, CaptureMode, ContourSource, FocalSource, PhotoWarning, Shape, VisionResult,
};

use measure::{Measurement, Outline};
use paper::PaperMapping;
use result::{BboxMm, HeightSource, Level, PaperResult, ProductResult, round2};
use segment::Pick;

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
    /// Where the user tapped the product, in photo pixels: keep the outline under it.
    pub tap_point_px: Option<[f64; 2]>,
    /// The product's height, typed by the user, in mm.
    pub height_mm: Option<f64>,
    /// The lens's focal length in 35 mm terms, in mm, from the photo's EXIF.
    pub focal_length_35mm: Option<f64>,
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
            tap_point_px: None,
            height_mm: None,
            focal_length_35mm: None,
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
    let mapping = PaperMapping::new(&quad, options.paper)?;
    let flat = mapping.flatten(image)?;
    let settings = segment::Settings {
        mode: options.mode,
        pick: pick_for((options.tap_point_px, image, &mapping))?,
        refine_with_grabcut: options.refine_with_grabcut,
        pen_width_mm: options.pen_width_mm,
    };
    let segmented = segment::cut_out(&flat, &settings)?;
    let measured = measure::measure(&segmented.mask, &flat)?;
    let confidence = measure::confidence(&measured, segmented.extra_blobs);
    let contour_px = mapping.to_photo_px(&measured.outline.paper_px())?;

    let mut result = VisionResult {
        mode: options.mode,
        paper: PaperResult {
            size: options.paper,
            corners_px: quad
                .corners
                .map(|p| [round2(f64::from(p.x)), round2(f64::from(p.y))]),
            px_per_mm: PX_PER_MM,
        },
        camera: camera::estimate((&mapping, image.size()?, options.focal_length_35mm))?,
        warnings: warnings(&WarningInputs {
            measured: &measured,
            extra_blobs: segmented.extra_blobs,
            confidence,
        }),
        products: vec![product_result(ProductParts {
            outline: &measured.outline,
            contour_px,
            confidence,
            source: ContourSource::Detected,
            height_mm: options.height_mm,
        })],
        level_used: Level::L1,
    };
    height::update_parallax_warning(&mut result)?;

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

/// What to keep on the paper: the largest outline, or the one at the user's tap.
fn pick_for(
    (tap, image, mapping): (Option<[f64; 2]>, &Mat, &PaperMapping),
) -> Result<Pick, VisionError> {
    let Some([x, y]) = tap else {
        return Ok(Pick::Largest);
    };
    let in_photo =
        (0.0..f64::from(image.cols())).contains(&x) && (0.0..f64::from(image.rows())).contains(&y);
    if !in_photo {
        return Err(VisionError::InvalidInput {
            field: "tap_point_px",
            value: format!("[{x}, {y}]"),
            allowed: input::TAP_ALLOWED,
        });
    }
    // A tap on the table has nothing to pick: products lie on the paper.
    let [paper_x, paper_y] = mapping
        .paper_point([x, y])?
        .ok_or(VisionError::NothingAtTap)?;
    Ok(Pick::At(Point2f::new(paper_x as f32, paper_y as f32)))
}

/// What one product of the result is made from.
pub(crate) struct ProductParts<'a> {
    pub outline: &'a Outline,
    /// The outline in photo pixels.
    pub contour_px: Vec<[f64; 2]>,
    pub confidence: f64,
    pub source: ContourSource,
    pub height_mm: Option<f64>,
}

/// The MVP's single product, rounded for output.
pub(crate) fn product_result(parts: ProductParts) -> ProductResult {
    let outline = parts.outline;
    let rounded = |points: &[[f64; 2]]| points.iter().map(|point| point.map(round2)).collect();
    ProductResult {
        id: "p1".to_owned(),
        shape: outline.shape,
        confidence: round2(parts.confidence),
        bbox_mm: BboxMm {
            l: round2(outline.length_mm),
            w: round2(outline.width_mm),
        },
        diameter_mm: outline.diameter_mm.map(round2),
        angle_deg: round2(outline.angle_deg),
        center_mm: outline.center_mm.map(round2),
        area_mm2: round2(outline.area_mm2),
        contour_mm: rounded(&outline.contour_mm),
        contour_px: rounded(&parts.contour_px),
        contour_source: parts.source,
        height_mm: parts.height_mm.map(round2),
        height_source: HeightSource::User,
    }
}

struct WarningInputs<'a> {
    measured: &'a Measurement,
    extra_blobs: usize,
    confidence: f64,
}

/// Hints about the photo (CLAUDE.md §5.5 `warnings`). The tall-product warning comes from
/// [`height::update_parallax_warning`].
fn warnings(inputs: &WarningInputs) -> Vec<PhotoWarning> {
    let mut list = Vec::new();
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
