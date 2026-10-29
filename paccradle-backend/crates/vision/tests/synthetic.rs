//! Pipeline tests on generated photos with known sizes (`common`). Real-photo accuracy is
//! measured separately with the M0-11 report.

mod common;

use std::error::Error;

use common::{
    Drawing, PHOTO_SIZE, SIDEWAYS_QUAD, TABLE_BGR, UPRIGHT_QUAD, assert_close, bgr, paper_image,
    photograph,
};
use opencv::core::{self, Mat, Size};
use vision::{AnalyzeOptions, CaptureMode, PaperSize, Shape, VisionError, analyze};

type TestResult = Result<(), Box<dyn Error>>;

fn options(paper: PaperSize, mode: CaptureMode) -> AnalyzeOptions {
    AnalyzeOptions {
        paper,
        mode,
        ..AnalyzeOptions::default()
    }
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
