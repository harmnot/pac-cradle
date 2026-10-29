//! Generated photos with known sizes, shared by the pipeline tests: a top-down paper image
//! is drawn at 10 px/mm, then "photographed" with a perspective tilt onto a dark table.
#![allow(dead_code)]

use opencv::core::{self, Mat, Point, Point2f, Scalar, Size, Vector};
use opencv::imgproc;
use opencv::prelude::*;
use vision::{PX_PER_MM, PaperSize};

/// Clean synthetic photos should land well inside the §5.6 targets.
pub const TOLERANCE_MM: f64 = 0.3;
pub const PAPER_BGR: (f64, f64, f64) = (232.0, 236.0, 238.0);
pub const TABLE_BGR: (f64, f64, f64) = (52.0, 58.0, 66.0);
pub const PRODUCT_BGR: (f64, f64, f64) = (40.0, 90.0, 170.0);
pub const INK_BGR: (f64, f64, f64) = (140.0, 45.0, 25.0);
pub const PHOTO_SIZE: (i32, i32) = (4000, 3000);
/// OpenCV drawing with 4 fractional bits for sub-pixel positions.
pub const SHIFT: i32 = 4;
pub const SHIFT_SCALE: f64 = 16.0;

/// Where the paper's own top-left, top-right, bottom-right and bottom-left corners land in
/// the photo.
///
/// Upright in a landscape photo, top edge farther away (narrower).
pub const UPRIGHT_QUAD: [(f32, f32); 4] = [
    (1180.0, 200.0),
    (2860.0, 240.0),
    (2980.0, 2800.0),
    (1050.0, 2760.0),
];
/// Turned a quarter clockwise: the paper's short top edge runs down the right of the photo.
pub const SIDEWAYS_QUAD: [(f32, f32); 4] = [
    (3420.0, 380.0),
    (3500.0, 2520.0),
    (460.0, 2580.0),
    (520.0, 420.0),
];

pub enum Drawing {
    /// A filled product of the given diameter.
    Disc {
        center_mm: (f64, f64),
        diameter_mm: f64,
    },
    /// A filled rotated box.
    Box {
        center_mm: (f64, f64),
        length_mm: f64,
        width_mm: f64,
        angle_deg: f64,
    },
    /// A pen line traced around a round product: line centre on the product edge.
    PenCircle {
        center_mm: (f64, f64),
        diameter_mm: f64,
        pen_mm: f64,
    },
}

pub fn bgr(color: (f64, f64, f64)) -> Scalar {
    Scalar::new(color.0, color.1, color.2, 0.0)
}

pub fn shifted(x_mm: f64, y_mm: f64) -> Point {
    Point::new(
        (x_mm * PX_PER_MM * SHIFT_SCALE).round() as i32,
        (y_mm * PX_PER_MM * SHIFT_SCALE).round() as i32,
    )
}

/// The paper seen straight from above at 10 px/mm, with one drawing on it.
pub fn paper_image(paper: PaperSize, drawing: &Drawing) -> opencv::Result<Mat> {
    let mut image = Mat::new_size_with_default(paper.size_px(), core::CV_8UC3, bgr(PAPER_BGR))?;
    match *drawing {
        Drawing::Disc {
            center_mm,
            diameter_mm,
        } => imgproc::circle(
            &mut image,
            shifted(center_mm.0, center_mm.1),
            (diameter_mm / 2.0 * PX_PER_MM * SHIFT_SCALE).round() as i32,
            bgr(PRODUCT_BGR),
            imgproc::FILLED,
            imgproc::LINE_AA,
            SHIFT,
        )?,
        Drawing::Box {
            center_mm,
            length_mm,
            width_mm,
            angle_deg,
        } => {
            let (sin, cos) = angle_deg.to_radians().sin_cos();
            let corners = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)].map(|(sx, sy)| {
                let (dx, dy) = (sx * length_mm / 2.0, sy * width_mm / 2.0);
                shifted(
                    center_mm.0 + dx * cos - dy * sin,
                    center_mm.1 + dx * sin + dy * cos,
                )
            });
            let polygon = Vector::<Vector<Point>>::from_iter([Vector::from_iter(corners)]);
            imgproc::fill_poly(
                &mut image,
                &polygon,
                bgr(PRODUCT_BGR),
                imgproc::LINE_AA,
                SHIFT,
                Point::default(),
            )?;
        }
        Drawing::PenCircle {
            center_mm,
            diameter_mm,
            pen_mm,
        } => imgproc::circle(
            &mut image,
            shifted(center_mm.0, center_mm.1),
            (diameter_mm / 2.0 * PX_PER_MM * SHIFT_SCALE).round() as i32,
            bgr(INK_BGR),
            (pen_mm * PX_PER_MM).round() as i32,
            imgproc::LINE_AA,
            SHIFT,
        )?,
    }
    Ok(image)
}

/// Places the paper image onto a table photo with a perspective transform.
pub fn photograph(paper_image: &Mat, quad: [(f32, f32); 4]) -> opencv::Result<Mat> {
    let (width, height) = (paper_image.cols() as f32, paper_image.rows() as f32);
    let source = Vector::<Point2f>::from_iter([
        Point2f::new(0.0, 0.0),
        Point2f::new(width, 0.0),
        Point2f::new(width, height),
        Point2f::new(0.0, height),
    ]);
    let target = Vector::<Point2f>::from_iter(quad.map(|(x, y)| Point2f::new(x, y)));
    let transform = imgproc::get_perspective_transform_def(&source, &target)?;
    let mut photo = Mat::new_size_with_default(
        Size::new(PHOTO_SIZE.0, PHOTO_SIZE.1),
        core::CV_8UC3,
        bgr(TABLE_BGR),
    )?;
    imgproc::warp_perspective(
        paper_image,
        &mut photo,
        &transform,
        Size::new(PHOTO_SIZE.0, PHOTO_SIZE.1),
        imgproc::INTER_LINEAR,
        core::BORDER_TRANSPARENT,
        Scalar::default(),
    )?;
    Ok(photo)
}

pub fn assert_close(label: &str, measured: f64, expected: f64) {
    let error = (measured - expected).abs();
    assert!(
        error <= TOLERANCE_MM,
        "{label}: measured {measured:.2} mm, expected {expected:.2} mm (error {error:.2} mm)"
    );
}
