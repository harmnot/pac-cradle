//! Find the A4/Letter sheet in a photo and flatten it to a top-down view
//! (CLAUDE.md §5.3 steps 1–3).

use std::str::FromStr;

use opencv::core::{self, Mat, Point, Point2f, Size, TermCriteria, Vector};
use opencv::imgproc;
use opencv::prelude::*;
use serde::Serialize;

use crate::error::VisionError;

/// Resolution of the flattened paper image: 10 px per mm, so 1 px = 0.1 mm.
pub const PX_PER_MM: f64 = 10.0;

/// Long edge of the downscaled working copy used to find the paper.
const WORKING_LONG_EDGE_PX: f64 = 2000.0;
/// The paper must cover at least this share of the photo.
const MIN_PAPER_AREA_RATIO: f64 = 0.20;
/// `approxPolyDP` tolerances tried in order, as a share of the contour perimeter.
const APPROX_EPSILON_RATIOS: [f64; 5] = [0.01, 0.02, 0.03, 0.04, 0.05];
/// All 4 corners must be inside the photo (CLAUDE.md §5.1), at least this far from its border
/// in working-copy pixels; otherwise the "quad" is just the photo frame.
const FRAME_MARGIN_PX: i32 = 3;
/// Canny thresholds for the edge-based paper search.
const CANNY_LOW: f64 = 50.0;
const CANNY_HIGH: f64 = 150.0;
/// Corner refinement: half window in full-resolution pixels, and how far a corner may move.
const SUBPIX_HALF_WINDOW: i32 = 5;
const SUBPIX_MAX_ITERATIONS: i32 = 40;
const SUBPIX_EPSILON: f64 = 0.01;
const MAX_SUBPIX_SHIFT_PX: f32 = 8.0;

/// Paper sheet sizes the app accepts (CLAUDE.md §3 step 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum PaperSize {
    /// ISO A4, 210 × 297 mm.
    A4,
    /// US Letter, 215.9 × 279.4 mm.
    Letter,
}

impl PaperSize {
    /// Width and height in millimetres, portrait.
    pub fn size_mm(self) -> (f64, f64) {
        match self {
            Self::A4 => (210.0, 297.0),
            Self::Letter => (215.9, 279.4),
        }
    }

    /// Width and height of the flattened image in pixels (A4: 2100 × 2970).
    pub fn size_px(self) -> Size {
        let (width_mm, height_mm) = self.size_mm();
        Size::new(mm_to_px(width_mm), mm_to_px(height_mm))
    }
}

impl FromStr for PaperSize {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text.to_ascii_lowercase().as_str() {
            "a4" => Ok(Self::A4),
            "letter" => Ok(Self::Letter),
            other => Err(format!("unknown paper size `{other}` (use a4 or letter)")),
        }
    }
}

/// The paper's corners in the photo, in portrait order:
/// top-left, top-right, bottom-right, bottom-left of the sheet.
#[derive(Debug, Clone, Copy)]
pub struct PaperQuad {
    /// Corner positions in original photo pixels.
    pub corners: [Point2f; 4],
}

/// Rounds a length in millimetres to whole pixels of the flattened image.
pub fn mm_to_px(mm: f64) -> i32 {
    (mm * PX_PER_MM).round() as i32
}

/// Finds the paper: the biggest convex 4-corner shape covering more than 20 % of the photo.
///
/// Works on a ~2000 px copy, then refines the corners on the full-resolution photo.
pub fn find_paper(image: &Mat) -> Result<PaperQuad, VisionError> {
    let scale = working_scale(image);
    let small = resized(image, scale)?;
    let mut blurred = Mat::default();
    imgproc::gaussian_blur_def(&to_gray(&small)?, &mut blurred, Size::new(5, 5), 0.0)?;

    let min_area = MIN_PAPER_AREA_RATIO * f64::from(small.rows()) * f64::from(small.cols());
    let bright = largest_quad(&bright_region_mask(&blurred)?, min_area)?;
    let edges = largest_quad(&edge_mask(&blurred)?, min_area)?;
    let (_, corners) = [bright, edges]
        .into_iter()
        .flatten()
        .max_by(|a, b| a.0.total_cmp(&b.0))
        .ok_or(VisionError::PaperNotFound)?;

    let full_scale = corners.map(|p| Point2f::new(p.x / scale as f32, p.y / scale as f32));
    let refined = refine_corners(&to_gray(image)?, full_scale)?;
    Ok(PaperQuad {
        corners: portrait_order(order_corners(refined)),
    })
}

/// Warps the paper to a top-down portrait view at 10 px/mm (A4 becomes 2100 × 2970 px).
pub fn flatten(image: &Mat, quad: &PaperQuad, paper: PaperSize) -> Result<Mat, VisionError> {
    let size = paper.size_px();
    let (width, height) = (size.width as f32, size.height as f32);
    let source = Vector::<Point2f>::from_iter(quad.corners);
    let target = Vector::<Point2f>::from_iter([
        Point2f::new(0.0, 0.0),
        Point2f::new(width, 0.0),
        Point2f::new(width, height),
        Point2f::new(0.0, height),
    ]);
    let transform = imgproc::get_perspective_transform_def(&source, &target)?;
    let mut flat = Mat::default();
    imgproc::warp_perspective_def(image, &mut flat, &transform, size)?;
    Ok(flat)
}

/// Converts a BGR image to grayscale.
pub fn to_gray(image: &Mat) -> Result<Mat, VisionError> {
    let mut gray = Mat::default();
    imgproc::cvt_color_def(image, &mut gray, imgproc::COLOR_BGR2GRAY)?;
    Ok(gray)
}

fn working_scale(image: &Mat) -> f64 {
    let long_edge = f64::from(image.rows().max(image.cols()));
    if long_edge <= WORKING_LONG_EDGE_PX {
        1.0
    } else {
        WORKING_LONG_EDGE_PX / long_edge
    }
}

fn resized(image: &Mat, scale: f64) -> Result<Mat, VisionError> {
    if scale >= 1.0 {
        return Ok(image.try_clone()?);
    }
    let mut small = Mat::default();
    imgproc::resize(
        image,
        &mut small,
        Size::default(),
        scale,
        scale,
        imgproc::INTER_AREA,
    )?;
    Ok(small)
}

/// The paper is the brightest large region on a darker surface (CLAUDE.md §5.1).
fn bright_region_mask(blurred: &Mat) -> Result<Mat, VisionError> {
    let mut mask = Mat::default();
    imgproc::threshold(
        blurred,
        &mut mask,
        0.0,
        255.0,
        imgproc::THRESH_BINARY | imgproc::THRESH_OTSU,
    )?;
    Ok(mask)
}

/// Fallback when the background is bright too: closed Canny edges.
fn edge_mask(blurred: &Mat) -> Result<Mat, VisionError> {
    let mut edges = Mat::default();
    imgproc::canny_def(blurred, &mut edges, CANNY_LOW, CANNY_HIGH)?;
    let kernel = imgproc::get_structuring_element_def(imgproc::MORPH_ELLIPSE, Size::new(3, 3))?;
    let mut closed = Mat::default();
    imgproc::morphology_ex_def(&edges, &mut closed, imgproc::MORPH_CLOSE, &kernel)?;
    Ok(closed)
}

/// Returns the area and corners of the biggest convex quadrilateral outline in a mask.
fn largest_quad(mask: &Mat, min_area: f64) -> Result<Option<(f64, [Point2f; 4])>, VisionError> {
    let mut contours = Vector::<Vector<Point>>::new();
    imgproc::find_contours_def(
        mask,
        &mut contours,
        imgproc::RETR_EXTERNAL,
        imgproc::CHAIN_APPROX_SIMPLE,
    )?;
    let mut best: Option<(f64, [Point2f; 4])> = None;
    for contour in contours.iter() {
        if imgproc::contour_area_def(&contour)? < min_area {
            continue;
        }
        let Some(quad) = approximate_quad(&contour)? else {
            continue;
        };
        if touches_frame(&quad, mask.size()?) {
            continue;
        }
        let area = imgproc::contour_area_def(&quad)?;
        if area >= min_area && best.as_ref().is_none_or(|(best_area, _)| area > *best_area) {
            best = Some((area, quad_points(&quad)?));
        }
    }
    Ok(best)
}

/// Simplifies a contour until it has exactly 4 corners and is convex.
fn approximate_quad(contour: &Vector<Point>) -> Result<Option<Vector<Point>>, VisionError> {
    let perimeter = imgproc::arc_length(contour, true)?;
    for ratio in APPROX_EPSILON_RATIOS {
        let mut approx = Vector::<Point>::new();
        imgproc::approx_poly_dp(contour, &mut approx, ratio * perimeter, true)?;
        if approx.len() == 4 && imgproc::is_contour_convex(&approx)? {
            return Ok(Some(approx));
        }
    }
    Ok(None)
}

fn touches_frame(quad: &Vector<Point>, size: Size) -> bool {
    quad.iter().any(|p| {
        p.x < FRAME_MARGIN_PX
            || p.y < FRAME_MARGIN_PX
            || p.x >= size.width - FRAME_MARGIN_PX
            || p.y >= size.height - FRAME_MARGIN_PX
    })
}

fn quad_points(quad: &Vector<Point>) -> Result<[Point2f; 4], VisionError> {
    let point = |index: usize| -> Result<Point2f, VisionError> {
        let p = quad.get(index)?;
        Ok(Point2f::new(p.x as f32, p.y as f32))
    };
    Ok([point(0)?, point(1)?, point(2)?, point(3)?])
}

/// Moves each corner to its sub-pixel position on the full photo, unless it jumps too far.
fn refine_corners(gray: &Mat, corners: [Point2f; 4]) -> Result<[Point2f; 4], VisionError> {
    let mut refined = Vector::<Point2f>::from_iter(corners);
    let criteria = TermCriteria::new(
        core::TermCriteria_COUNT + core::TermCriteria_EPS,
        SUBPIX_MAX_ITERATIONS,
        SUBPIX_EPSILON,
    )?;
    imgproc::corner_sub_pix(
        gray,
        &mut refined,
        Size::new(SUBPIX_HALF_WINDOW, SUBPIX_HALF_WINDOW),
        Size::new(-1, -1),
        criteria,
    )?;
    let mut result = corners;
    for (index, original) in corners.iter().enumerate() {
        let moved = refined.get(index)?;
        let shift = ((moved.x - original.x).powi(2) + (moved.y - original.y).powi(2)).sqrt();
        if shift <= MAX_SUBPIX_SHIFT_PX {
            result[index] = moved;
        }
    }
    Ok(result)
}

/// Orders corners as top-left, top-right, bottom-right, bottom-left in the photo.
fn order_corners(corners: [Point2f; 4]) -> [Point2f; 4] {
    let by = |key: fn(&Point2f) -> f32, pick_max: bool| -> Point2f {
        let mut sorted = corners;
        sorted.sort_by(|a, b| key(a).total_cmp(&key(b)));
        if pick_max { sorted[3] } else { sorted[0] }
    };
    let top_left = by(|p| p.x + p.y, false);
    let bottom_right = by(|p| p.x + p.y, true);
    let top_right = by(|p| p.y - p.x, false);
    let bottom_left = by(|p| p.y - p.x, true);
    [top_left, top_right, bottom_right, bottom_left]
}

/// Rotates the corner order when the sheet lies sideways, so its long edge maps to the
/// flattened image's height.
fn portrait_order(ordered: [Point2f; 4]) -> [Point2f; 4] {
    let [top_left, top_right, bottom_right, bottom_left] = ordered;
    let horizontal = distance(top_left, top_right) + distance(bottom_left, bottom_right);
    let vertical = distance(top_left, bottom_left) + distance(top_right, bottom_right);
    if horizontal > vertical {
        [top_right, bottom_right, bottom_left, top_left]
    } else {
        ordered
    }
}

fn distance(a: Point2f, b: Point2f) -> f32 {
    ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt()
}
