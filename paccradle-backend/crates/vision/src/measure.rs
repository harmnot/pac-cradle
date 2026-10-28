//! Measure a product mask in millimetres and score the confidence
//! (CLAUDE.md §5.3 steps 6–8).

use std::f64::consts::PI;

use opencv::core::{self, Mat, Point, Point2f, Vector};
use opencv::imgproc;
use opencv::prelude::*;

use crate::error::VisionError;
use crate::paper::{PX_PER_MM, mm_to_px, to_gray};
use crate::result::Shape;
use crate::segment::EDGE_IGNORE_MM;

/// Outline simplification tolerance: 0.2 mm (CLAUDE.md §5.3 step 7).
const SIMPLIFY_EPSILON_PX: f64 = 2.0;
/// Mask pixels sit on pixel centres; the real edge is half a pixel further out on each side.
const PIXEL_EDGE_CORRECTION_PX: f64 = 1.0;
/// Shape rules.
const ROUND_MIN_CIRCULARITY: f64 = 0.85;
const ROUND_MAX_ASPECT: f64 = 1.1;
const RECTANGULAR_MIN_FILL: f64 = 0.92;
/// A product closer than this to the ignored edge band "touches the edge".
const EDGE_TOUCH_MARGIN_PX: i32 = 5;
/// Confidence penalties (spike values; tune with the M0-11 accuracy report).
const TOUCH_PENALTY: f64 = 0.4;
const EXTRA_BLOB_PENALTY: f64 = 0.15;
const MAX_COUNTED_EXTRA_BLOBS: usize = 3;
const SOLID_ENOUGH: f64 = 0.8;
const WEAK_EDGE_GRADIENT: f64 = 40.0;
const WEAK_EDGE_PENALTY: f64 = 0.2;

/// Sizes and outline of one product, in millimetres on the flattened paper.
#[derive(Debug, Clone)]
pub struct Measurement {
    pub shape: Shape,
    /// Long and short side of the rotated bounding box.
    pub length_mm: f64,
    pub width_mm: f64,
    /// Smallest enclosing circle, for round shapes.
    pub diameter_mm: Option<f64>,
    pub angle_deg: f64,
    pub center_mm: [f64; 2],
    pub area_mm2: f64,
    /// Simplified outline relative to its own top-left corner.
    pub contour_mm: Vec<[f64; 2]>,
    /// Area ÷ convex hull area (1 = no dents).
    pub solidity: f64,
    /// 4π·area ÷ perimeter² (1 = perfect circle).
    pub circularity: f64,
    /// Mean gradient strength along the outline (sharp edges score higher).
    pub edge_strength: f64,
    pub touches_edge: bool,
}

/// Measures the largest outline in `mask`; `flat` is the flattened photo (for edge strength).
pub fn measure(mask: &Mat, flat: &Mat) -> Result<Measurement, VisionError> {
    let contour = largest_contour(mask)?.ok_or(VisionError::ProductNotFound)?;

    let rect = imgproc::min_area_rect(&contour)?;
    let side_a = f64::from(rect.size.width) + PIXEL_EDGE_CORRECTION_PX;
    let side_b = f64::from(rect.size.height) + PIXEL_EDGE_CORRECTION_PX;
    let (length_px, width_px) = (side_a.max(side_b), side_a.min(side_b));

    let area_px = f64::from(core::count_non_zero(mask)?);
    let mut simplified = Vector::<Point>::new();
    imgproc::approx_poly_dp(&contour, &mut simplified, SIMPLIFY_EPSILON_PX, true)?;
    let perimeter_px = imgproc::arc_length(&simplified, true)?;
    let circularity = 4.0 * PI * area_px / perimeter_px.powi(2);

    let mut hull = Vector::<Point>::new();
    imgproc::convex_hull_def(&contour, &mut hull)?;
    let hull_area = imgproc::contour_area_def(&hull)?;
    let solidity = if hull_area > 0.0 {
        imgproc::contour_area_def(&contour)? / hull_area
    } else {
        0.0
    };

    let mut circle_center = Point2f::default();
    let mut circle_radius = 0.0_f32;
    imgproc::min_enclosing_circle(&contour, &mut circle_center, &mut circle_radius)?;

    let shape = classify(ShapeStats {
        circularity,
        aspect: length_px / width_px,
        fill: area_px / (length_px * width_px),
    });
    let diameter_mm = (shape == Shape::Round)
        .then(|| (2.0 * f64::from(circle_radius) + PIXEL_EDGE_CORRECTION_PX) / PX_PER_MM);

    let bounds = imgproc::bounding_rect(&contour)?;
    let contour_mm = simplified
        .iter()
        .map(|p| {
            [
                f64::from(p.x - bounds.x) / PX_PER_MM,
                f64::from(p.y - bounds.y) / PX_PER_MM,
            ]
        })
        .collect();
    let edge_band_px = mm_to_px(EDGE_IGNORE_MM) + EDGE_TOUCH_MARGIN_PX;
    let touches_edge = bounds.x <= edge_band_px
        || bounds.y <= edge_band_px
        || bounds.x + bounds.width >= mask.cols() - edge_band_px
        || bounds.y + bounds.height >= mask.rows() - edge_band_px;

    Ok(Measurement {
        shape,
        length_mm: length_px / PX_PER_MM,
        width_mm: width_px / PX_PER_MM,
        diameter_mm,
        angle_deg: f64::from(rect.angle),
        center_mm: [
            f64::from(rect.center.x) / PX_PER_MM,
            f64::from(rect.center.y) / PX_PER_MM,
        ],
        area_mm2: area_px / (PX_PER_MM * PX_PER_MM),
        contour_mm,
        solidity,
        circularity,
        edge_strength: edge_strength(flat, &contour)?,
        touches_edge,
    })
}

/// Scores how much to trust a measurement, from 0 to 1 (CLAUDE.md §5.3 step 8).
///
/// Starts at 1 and subtracts for: touching the paper edge, extra objects, a dented outline
/// and a weak edge.
pub fn confidence(measurement: &Measurement, extra_blobs: usize) -> f64 {
    let mut score = 1.0;
    if measurement.touches_edge {
        score -= TOUCH_PENALTY;
    }
    score -= EXTRA_BLOB_PENALTY * extra_blobs.min(MAX_COUNTED_EXTRA_BLOBS) as f64;
    if measurement.solidity < SOLID_ENOUGH {
        score -= SOLID_ENOUGH - measurement.solidity;
    }
    if measurement.edge_strength < WEAK_EDGE_GRADIENT {
        score -= WEAK_EDGE_PENALTY;
    }
    score.clamp(0.0, 1.0)
}

struct ShapeStats {
    circularity: f64,
    aspect: f64,
    fill: f64,
}

fn classify(stats: ShapeStats) -> Shape {
    if stats.circularity >= ROUND_MIN_CIRCULARITY && stats.aspect <= ROUND_MAX_ASPECT {
        return Shape::Round;
    }
    if stats.fill >= RECTANGULAR_MIN_FILL {
        return Shape::Rectangular;
    }
    Shape::Freeform
}

fn largest_contour(mask: &Mat) -> Result<Option<Vector<Point>>, VisionError> {
    let mut contours = Vector::<Vector<Point>>::new();
    imgproc::find_contours_def(
        mask,
        &mut contours,
        imgproc::RETR_EXTERNAL,
        imgproc::CHAIN_APPROX_NONE,
    )?;
    let mut best: Option<(f64, Vector<Point>)> = None;
    for contour in contours.iter() {
        let area = imgproc::contour_area_def(&contour)?;
        if best.as_ref().is_none_or(|(best_area, _)| area > *best_area) {
            best = Some((area, contour));
        }
    }
    Ok(best.map(|(_, contour)| contour))
}

/// Mean Sobel gradient magnitude of the photo along the outline.
fn edge_strength(flat: &Mat, contour: &Vector<Point>) -> Result<f64, VisionError> {
    let gray = to_gray(flat)?;
    let mut gradient_x = Mat::default();
    let mut gradient_y = Mat::default();
    imgproc::sobel_def(&gray, &mut gradient_x, core::CV_32F, 1, 0)?;
    imgproc::sobel_def(&gray, &mut gradient_y, core::CV_32F, 0, 1)?;
    let mut magnitude = Mat::default();
    core::magnitude(&gradient_x, &gradient_y, &mut magnitude)?;

    if contour.is_empty() {
        return Ok(0.0);
    }
    let mut total = 0.0;
    for point in contour.iter() {
        total += f64::from(*magnitude.at_2d::<f32>(point.y, point.x)?);
    }
    Ok(total / contour.len() as f64)
}
