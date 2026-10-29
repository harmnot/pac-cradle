//! M1-C3: manual corrections (CLAUDE.md §3 step 4). A tap re-runs detection and keeps the
//! outline under it, in photo and trace mode; taps off the product, off the paper or off
//! the photo are refused. Detected outlines come in photo pixels too, lying over the
//! product. An edited outline is measured exactly as drawn, from the stored result alone,
//! and bad outlines are refused.
// The helpers below are test code too; clippy.toml only exempts `#[test]` functions.
#![allow(clippy::unwrap_used)]

mod common;

use std::error::Error;

use common::{
    Drawing, UPRIGHT_QUAD, assert_close, encode, paper_image, paper_image_with, paper_mm, photo_px,
    photograph,
};
use opencv::core::Mat;
use serde_json::json;
use vision::{
    AnalyzeOptions, CaptureInput, CaptureMode, ContourSource, MAX_EDIT_POINTS, PaperSize,
    PhotoWarning, Shape, VisionError, VisionResult, accept_edit, analyze,
};

type TestResult = Result<(), Box<dyn Error>>;

const A4: PaperSize = PaperSize::A4;
/// An edited outline skips the photo, so only the paper corners' sub-pixel error is left.
const EDIT_TOLERANCE_MM: f64 = 0.1;
const BIG: ([f64; 2], f64) = ([105.0, 90.0], 60.0);
const SMALL: ([f64; 2], f64) = ([105.0, 220.0], 40.0);

fn disc((center, diameter_mm): ([f64; 2], f64)) -> Drawing {
    Drawing::Disc {
        center_mm: (center[0], center[1]),
        diameter_mm,
    }
}

fn pen_circle((center, diameter_mm): ([f64; 2], f64)) -> Drawing {
    Drawing::PenCircle {
        center_mm: (center[0], center[1]),
        diameter_mm,
        pen_mm: 0.5,
    }
}

/// A 60 mm and a 40 mm product on one sheet.
fn two_discs() -> opencv::Result<Mat> {
    photograph(
        &paper_image_with(A4, &[disc(BIG), disc(SMALL)])?,
        UPRIGHT_QUAD,
    )
}

/// Where a point on the paper, in mm, appears in the photo.
fn photo_point(point_mm: [f64; 2]) -> [f64; 2] {
    photo_px((A4, UPRIGHT_QUAD, &[point_mm])).unwrap()[0]
}

fn options((mode, tap_point_px): (CaptureMode, Option<[f64; 2]>)) -> AnalyzeOptions {
    AnalyzeOptions {
        paper: A4,
        mode,
        tap_point_px,
        ..AnalyzeOptions::default()
    }
}

fn diameter(result: &VisionResult) -> f64 {
    result.products[0].diameter_mm.unwrap()
}

fn assert_near((label, measured, expected): (&str, f64, f64)) {
    let error = (measured - expected).abs();
    assert!(
        error <= EDIT_TOLERANCE_MM,
        "{label}: measured {measured:.3} mm, expected {expected:.3} mm (error {error:.3} mm)"
    );
}

#[test]
fn a_tap_picks_the_tapped_product() -> TestResult {
    let photo = two_discs()?;
    // Without a tap the biggest wins, and the other counts against it.
    let untapped = analyze(&photo, &options((CaptureMode::Photo, None)))?.result;
    assert_close("largest", diameter(&untapped), BIG.1);
    assert!(
        untapped
            .warnings
            .contains(&PhotoWarning::OtherObjects { count: 1 })
    );
    assert!(untapped.products[0].confidence < 0.9);

    // A tap on the small one picks it. The big one no longer counts: the user chose.
    let tap = Some(photo_point(SMALL.0));
    let tapped = analyze(&photo, &options((CaptureMode::Photo, tap)))?.result;
    let product = &tapped.products[0];
    assert_close("tapped", diameter(&tapped), SMALL.1);
    assert_eq!(product.contour_source, ContourSource::Detected);
    assert!(product.confidence >= 0.85, "{}", product.confidence);
    assert_eq!(tapped.warnings, vec![PhotoWarning::TallProductLooksBigger]);

    // A tap that just misses, 5 mm past the small one's edge, still picks it.
    let near = [SMALL.0[0] + SMALL.1 / 2.0 + 5.0, SMALL.0[1]];
    let missed = analyze(
        &photo,
        &options((CaptureMode::Photo, Some(photo_point(near)))),
    )?;
    assert_close("near miss", diameter(&missed.result), SMALL.1);
    Ok(())
}

#[test]
fn a_tap_reaches_run_through_the_capture_input() -> TestResult {
    let bytes = encode((&two_discs()?, ".jpg"))?;
    let input: CaptureInput = serde_json::from_value(json!({
        "paper": "A4",
        "mode": "photo",
        "tap_point_px": photo_point(SMALL.0),
    }))?;
    assert!(input.tap_point_px.is_some());
    assert_close("run", diameter(&vision::run((&input, &bytes))?), SMALL.1);

    // A first run's input has no tap, and stores none.
    let first = CaptureInput {
        tap_point_px: None,
        ..input
    };
    assert!(serde_json::to_value(first)?.get("tap_point_px").is_none());
    Ok(())
}

#[test]
fn a_tap_on_nothing_is_refused() -> TestResult {
    let photo = two_discs()?;
    // Bare paper, more than 50 mm from either product; then the table beside the paper.
    for tap in [photo_point([40.0, 160.0]), [100.0, 100.0]] {
        let error = analyze(&photo, &options((CaptureMode::Photo, Some(tap)))).err();
        let error = error.ok_or("a tap on nothing was accepted")?;
        assert!(
            matches!(error, VisionError::NothingAtTap),
            "{tap:?}: {error}"
        );
        assert_eq!(error.code(), "CAPTURE_NOTHING_AT_TAP");
        assert!(error.is_users_to_fix());
    }
    Ok(())
}

#[test]
fn a_tap_must_lie_in_the_photo_and_be_a_number() -> TestResult {
    let photo = two_discs()?;
    for tap in [[5000.0, 100.0], [-1.0, 100.0], [100.0, 3000.0]] {
        let error = analyze(&photo, &options((CaptureMode::Photo, Some(tap)))).err();
        assert!(
            matches!(
                error,
                Some(VisionError::InvalidInput {
                    field: "tap_point_px",
                    ..
                })
            ),
            "{tap:?}"
        );
    }
    // Checked before the file is even looked at.
    let input = CaptureInput {
        paper: A4,
        mode: CaptureMode::Photo,
        pen_width_mm: vision::DEFAULT_PEN_WIDTH_MM,
        tap_point_px: Some([f64::NAN, 10.0]),
        height_mm: None,
        focal_length_35mm: None,
    };
    let error = vision::run((&input, b"not an image")).unwrap_err();
    assert!(matches!(
        error,
        VisionError::InvalidInput {
            field: "tap_point_px",
            ..
        }
    ));
    Ok(())
}

#[test]
fn a_tap_picks_the_tapped_pen_loop() -> TestResult {
    let traced = paper_image_with(A4, &[pen_circle(BIG), pen_circle(SMALL)])?;
    let photo = photograph(&traced, UPRIGHT_QUAD)?;
    let untapped = analyze(&photo, &options((CaptureMode::Trace, None)))?.result;
    assert_close("largest loop", diameter(&untapped), BIG.1);
    assert!(
        untapped
            .warnings
            .contains(&PhotoWarning::OtherObjects { count: 1 })
    );

    let tap = Some(photo_point(SMALL.0));
    let tapped = analyze(&photo, &options((CaptureMode::Trace, tap)))?.result;
    assert_close("tapped loop", diameter(&tapped), SMALL.1);
    assert!(tapped.warnings.is_empty(), "{:?}", tapped.warnings);

    let off = Some(photo_point([40.0, 160.0]));
    let error = analyze(&photo, &options((CaptureMode::Trace, off))).err();
    assert!(matches!(error, Some(VisionError::NothingAtTap)));
    Ok(())
}

/// A 52 mm product at (105, 150) mm on A4, and its detected result.
fn detected_disc() -> Result<VisionResult, Box<dyn Error>> {
    let photo = photograph(
        &paper_image(A4, &disc(([105.0, 150.0], 52.0)))?,
        UPRIGHT_QUAD,
    )?;
    Ok(analyze(&photo, &options((CaptureMode::Photo, None)))?.result)
}

#[test]
fn detected_outlines_come_in_photo_pixels_over_the_product() -> TestResult {
    let result = detected_disc()?;
    let product = &result.products[0];
    assert_eq!(product.contour_source, ContourSource::Detected);
    assert_eq!(product.contour_px.len(), product.contour_mm.len());
    // Mapped back with the photo's true geometry, every point lies on the product's edge.
    for [x, y] in paper_mm((A4, UPRIGHT_QUAD, &product.contour_px))? {
        let radius = (x - 105.0).hypot(y - 150.0);
        assert_close("radius", radius, 26.0);
    }
    Ok(())
}

/// The corners of a `length` × `width` mm rectangle centred at `center`, turned by `angle_deg`.
fn rectangle((center, [length, width], angle_deg): ([f64; 2], [f64; 2], f64)) -> Vec<[f64; 2]> {
    let (sin, cos) = angle_deg.to_radians().sin_cos();
    [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
        .iter()
        .map(|(sx, sy)| {
            let (dx, dy) = (sx * length / 2.0, sy * width / 2.0);
            [
                center[0] + dx * cos - dy * sin,
                center[1] + dx * sin + dy * cos,
            ]
        })
        .collect()
}

#[test]
fn an_edited_outline_is_measured_as_drawn() -> TestResult {
    let previous = detected_disc()?;
    // Nothing like the photo's disc, so nothing can come from detection.
    let corners = rectangle(([100.0, 160.0], [70.0, 40.0], 10.0));
    let outline = photo_px((A4, UPRIGHT_QUAD, &corners))?;
    let edited = accept_edit((&previous, &outline))?;

    let product = &edited.products[0];
    assert_eq!(product.contour_source, ContourSource::User);
    assert_eq!(product.confidence, 1.0);
    assert_eq!(product.shape, Shape::Rectangular);
    assert_near(("length", product.bbox_mm.l, 70.0));
    assert_near(("width", product.bbox_mm.w, 40.0));
    assert_near(("centre x", product.center_mm[0], 100.0));
    assert_near(("centre y", product.center_mm[1], 160.0));
    assert!(
        (product.area_mm2 - 2800.0).abs() < 10.0,
        "{}",
        product.area_mm2
    );
    // As drawn: the same 4 corners, the photo points echoed back.
    assert_eq!(product.contour_mm.len(), 4);
    let rounded: Vec<[f64; 2]> = outline
        .iter()
        .map(|point| point.map(|value| (value * 100.0).round() / 100.0))
        .collect();
    assert_eq!(product.contour_px, rounded);
    // The capture's paper, mode and level stay; only the parallax warning is left.
    assert_eq!(edited.paper, previous.paper);
    assert_eq!(edited.mode, CaptureMode::Photo);
    assert_eq!(edited.level_used, previous.level_used);
    assert_eq!(edited.warnings, vec![PhotoWarning::TallProductLooksBigger]);

    // A trace has no parallax, so no warnings at all.
    let traced = VisionResult {
        mode: CaptureMode::Trace,
        ..previous
    };
    assert!(accept_edit((&traced, &outline))?.warnings.is_empty());
    Ok(())
}

#[test]
fn an_outline_sent_back_unchanged_measures_as_detected() -> TestResult {
    let drawing = Drawing::Box {
        center_mm: (100.0, 160.0),
        length_mm: 80.0,
        width_mm: 30.0,
        angle_deg: 25.0,
    };
    let photo = photograph(&paper_image(A4, &drawing)?, UPRIGHT_QUAD)?;
    let detected = analyze(&photo, &options((CaptureMode::Photo, None)))?.result;
    let before = &detected.products[0];
    let edited = accept_edit((&detected, &before.contour_px))?;
    let after = &edited.products[0];

    assert_eq!(after.shape, before.shape);
    assert_eq!(after.contour_px, before.contour_px);
    // Detected sizes come from every edge pixel, plus half a pixel at each edge. The
    // outline keeps fewer points (ε 0.2 mm) and is the edge as drawn, so sent back it
    // measures a little smaller: 80.25 × 30.26 became 80.08 × 30.02 here (true: 80 × 30).
    for (label, was, now) in [
        ("length", before.bbox_mm.l, after.bbox_mm.l),
        ("width", before.bbox_mm.w, after.bbox_mm.w),
    ] {
        assert!(
            (0.0..=0.3).contains(&(was - now)),
            "{label}: detected {was}, sent back {now}"
        );
    }
    assert_near(("centre x", after.center_mm[0], before.center_mm[0]));
    assert_near(("centre y", after.center_mm[1], before.center_mm[1]));
    Ok(())
}

fn edit_error((previous, outline): (&VisionResult, &[[f64; 2]])) -> VisionError {
    accept_edit((previous, outline)).unwrap_err()
}

fn is_invalid_outline(error: &VisionError) -> bool {
    matches!(
        error,
        VisionError::InvalidInput {
            field: "edited_contour_px",
            ..
        }
    )
}

#[test]
fn bad_edited_outlines_are_refused() -> TestResult {
    let previous = detected_disc()?;
    let square = |center: [f64; 2], side: f64| {
        photo_px((A4, UPRIGHT_QUAD, &rectangle((center, [side, side], 0.0)))).unwrap()
    };
    let good = square([105.0, 150.0], 40.0);

    // Too few points, too many, and a point that isn't a number.
    let too_many = vec![good[0]; MAX_EDIT_POINTS + 1];
    let not_a_number = [good[0], good[1], [f64::NAN, good[2][1]], good[3]];
    // A corner on the table, and a 3 mm square.
    let off_paper = [good[0], good[1], good[2], [100.0, 100.0]];
    let tiny = square([105.0, 150.0], 3.0);
    let outlines: [&[[f64; 2]]; 5] = [&good[..2], &too_many, &not_a_number, &off_paper, &tiny];
    for outline in outlines {
        let error = edit_error((&previous, outline));
        assert!(is_invalid_outline(&error), "{error}");
        assert_eq!(error.code(), "CAPTURE_INVALID_INPUT");
    }

    // Two corners swapped: the outline crosses itself.
    let bow_tie = [good[0], good[2], good[1], good[3]];
    let error = edit_error((&previous, &bow_tie));
    assert!(
        matches!(error, VisionError::OutlineCrossesItself),
        "{error}"
    );
    assert_eq!(error.code(), "CAPTURE_OUTLINE_CROSSES_ITSELF");

    // Repeated points, and the first point again at the end, add nothing.
    let repeated = [good[0], good[1], good[1], good[2], good[3], good[0]];
    let product = &accept_edit((&previous, &repeated))?.products[0];
    assert_eq!(product.contour_px.len(), 4);
    assert_near(("side", product.bbox_mm.l, 40.0));
    Ok(())
}
