//! Pipeline tests on generated photos with known sizes: a top-down paper image is drawn at
//! 10 px/mm, then "photographed" with a perspective tilt onto a dark table.
//! Real-photo accuracy is measured separately with the M0-11 report.

use std::error::Error;

use opencv::core::{self, Mat, Point, Point2f, Scalar, Size, Vector};
use opencv::imgproc;
use opencv::prelude::*;
use vision::{AnalyzeOptions, CaptureMode, PX_PER_MM, PaperSize, Shape, VisionError, analyze};

type TestResult = Result<(), Box<dyn Error>>;

/// Clean synthetic photos should land well inside the §5.6 targets.
const TOLERANCE_MM: f64 = 0.3;
const PAPER_BGR: (f64, f64, f64) = (232.0, 236.0, 238.0);
const TABLE_BGR: (f64, f64, f64) = (52.0, 58.0, 66.0);
const PRODUCT_BGR: (f64, f64, f64) = (40.0, 90.0, 170.0);
const INK_BGR: (f64, f64, f64) = (140.0, 45.0, 25.0);
const PHOTO_SIZE: (i32, i32) = (4000, 3000);
/// OpenCV drawing with 4 fractional bits for sub-pixel positions.
const SHIFT: i32 = 4;
const SHIFT_SCALE: f64 = 16.0;

/// Where the paper's own top-left, top-right, bottom-right and bottom-left corners land in
/// the photo.
///
/// Upright in a landscape photo, top edge farther away (narrower).
const UPRIGHT_QUAD: [(f32, f32); 4] = [
    (1180.0, 200.0),
    (2860.0, 240.0),
    (2980.0, 2800.0),
    (1050.0, 2760.0),
];
/// Turned a quarter clockwise: the paper's short top edge runs down the right of the photo.
const SIDEWAYS_QUAD: [(f32, f32); 4] = [
    (3420.0, 380.0),
    (3500.0, 2520.0),
    (460.0, 2580.0),
    (520.0, 420.0),
];

enum Drawing {
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

fn bgr(color: (f64, f64, f64)) -> Scalar {
    Scalar::new(color.0, color.1, color.2, 0.0)
}

fn shifted(x_mm: f64, y_mm: f64) -> Point {
    Point::new(
        (x_mm * PX_PER_MM * SHIFT_SCALE).round() as i32,
        (y_mm * PX_PER_MM * SHIFT_SCALE).round() as i32,
    )
}

/// The paper seen straight from above at 10 px/mm, with one drawing on it.
fn paper_image(paper: PaperSize, drawing: &Drawing) -> opencv::Result<Mat> {
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
fn photograph(paper_image: &Mat, quad: [(f32, f32); 4]) -> opencv::Result<Mat> {
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

fn options(paper: PaperSize, mode: CaptureMode) -> AnalyzeOptions {
    AnalyzeOptions {
        paper,
        mode,
        ..AnalyzeOptions::default()
    }
}

fn assert_close(label: &str, measured: f64, expected: f64) {
    let error = (measured - expected).abs();
    assert!(
        error <= TOLERANCE_MM,
        "{label}: measured {measured:.2} mm, expected {expected:.2} mm (error {error:.2} mm)"
    );
}

#[test]
fn photo_mode_measures_a_round_product() -> TestResult {
    let drawing = Drawing::Disc {
        center_mm: (105.0, 150.0),
        diameter_mm: 52.0,
    };
    let photo = photograph(&paper_image(PaperSize::A4, &drawing)?, UPRIGHT_QUAD)?;
    let result = analyze(&photo, &options(PaperSize::A4, CaptureMode::Photo))?.result;

    let product = &result.products[0];
    assert_eq!(product.shape, Shape::Round);
    assert_close("diameter", product.diameter_mm.ok_or("no diameter")?, 52.0);
    assert_close("length", product.bbox_mm.l, 52.0);
    assert_close("width", product.bbox_mm.w, 52.0);
    assert!(
        product.confidence >= 0.85,
        "confidence {}",
        product.confidence
    );
    Ok(())
}

#[test]
fn photo_mode_measures_a_rotated_box() -> TestResult {
    let drawing = Drawing::Box {
        center_mm: (100.0, 160.0),
        length_mm: 80.0,
        width_mm: 30.0,
        angle_deg: 25.0,
    };
    let photo = photograph(&paper_image(PaperSize::A4, &drawing)?, UPRIGHT_QUAD)?;
    let product = analyze(&photo, &options(PaperSize::A4, CaptureMode::Photo))?
        .result
        .products[0]
        .clone();

    assert_eq!(product.shape, Shape::Rectangular);
    assert_close("length", product.bbox_mm.l, 80.0);
    assert_close("width", product.bbox_mm.w, 30.0);
    Ok(())
}

#[test]
fn trace_mode_measures_the_traced_outline() -> TestResult {
    let drawing = Drawing::PenCircle {
        center_mm: (105.0, 140.0),
        diameter_mm: 60.0,
        pen_mm: 0.5,
    };
    let photo = photograph(&paper_image(PaperSize::A4, &drawing)?, UPRIGHT_QUAD)?;
    let result = analyze(&photo, &options(PaperSize::A4, CaptureMode::Trace))?.result;

    let product = &result.products[0];
    assert_eq!(product.shape, Shape::Round);
    assert_close("diameter", product.diameter_mm.ok_or("no diameter")?, 60.0);
    Ok(())
}

#[test]
fn finds_letter_paper_lying_sideways() -> TestResult {
    let drawing = Drawing::Disc {
        center_mm: (108.0, 140.0),
        diameter_mm: 40.0,
    };
    let photo = photograph(&paper_image(PaperSize::Letter, &drawing)?, SIDEWAYS_QUAD)?;
    let result = analyze(&photo, &options(PaperSize::Letter, CaptureMode::Photo))?.result;

    assert_close(
        "diameter",
        result.products[0].diameter_mm.ok_or("no diameter")?,
        40.0,
    );
    Ok(())
}

#[test]
fn a_photo_without_paper_is_rejected() -> TestResult {
    let photo = Mat::new_size_with_default(
        Size::new(PHOTO_SIZE.0, PHOTO_SIZE.1),
        core::CV_8UC3,
        bgr(TABLE_BGR),
    )?;
    let outcome = analyze(&photo, &AnalyzeOptions::default());
    assert!(matches!(outcome, Err(VisionError::PaperNotFound)));
    Ok(())
}
