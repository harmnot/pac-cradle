//! M1-C4: the typed height and the tall-product warning (CLAUDE.md §5.2 fix B, DEC-09).
//! The camera is worked out from the paper's corners and the lens, on photos from a known
//! pinhole camera, level and tilted, with and without the EXIF focal length. A tall
//! cylinder drawn as the camera would see it reaches past its edge by what the warning
//! says. Short products and traces get no warning; `set_height` matches a run with the
//! height, and edits keep it; heights and focal lengths are checked.
// The helpers below are test code too; clippy.toml only exempts `#[test]` functions.
#![allow(clippy::unwrap_used)]

mod common;

use std::error::Error;

use common::{Drawing, Pinhole, paper_image_with, paper_mm, photo_px, photograph};
use opencv::core::Mat;
use serde_json::json;
use vision::{
    AnalyzeOptions, CaptureInput, CaptureMode, FocalSource, PaperSize, PhotoWarning, VisionError,
    VisionResult, accept_edit, analyze, set_height,
};

type TestResult = Result<(), Box<dyn Error>>;

const A4: PaperSize = PaperSize::A4;
const CENTRE: [f64; 2] = [105.0, 148.5];
/// Straight above the middle of the sheet, 35 cm up, on a 26 mm phone lens.
const LEVEL: Pinhole = Pinhole {
    foot_mm: CENTRE,
    height_mm: 350.0,
    focal_35mm: 26.0,
};
/// 30 cm up, its foot 3 cm past the sheet's bottom edge: tilted about 31°.
const TILTED: Pinhole = Pinhole {
    foot_mm: [105.0, 330.0],
    height_mm: 300.0,
    focal_35mm: 26.0,
};

fn disc((center, diameter_mm): ([f64; 2], f64)) -> Drawing {
    Drawing::Disc {
        center_mm: (center[0], center[1]),
        diameter_mm,
    }
}

fn photo((camera, drawings): (&Pinhole, &[Drawing])) -> opencv::Result<Mat> {
    photograph(&paper_image_with(A4, drawings)?, camera.quad(A4))
}

/// What the run settings say about the height and the lens.
struct Run {
    mode: CaptureMode,
    height_mm: Option<f64>,
    focal_35mm: Option<f64>,
}

const PHOTO_WITH_LENS: Run = Run {
    mode: CaptureMode::Photo,
    height_mm: None,
    focal_35mm: Some(26.0),
};

fn measure((photo, run): (&Mat, Run)) -> Result<VisionResult, VisionError> {
    let options = AnalyzeOptions {
        paper: A4,
        mode: run.mode,
        height_mm: run.height_mm,
        focal_length_35mm: run.focal_35mm,
        ..AnalyzeOptions::default()
    };
    Ok(analyze(photo, &options)?.result)
}

fn reach_warning(result: &VisionResult) -> Option<(f64, f64)> {
    result.warnings.iter().find_map(|warning| match warning {
        PhotoWarning::OutlineMayBeBigger {
            height_mm,
            up_to_mm,
        } => Some((*height_mm, *up_to_mm)),
        _ => None,
    })
}

fn has_parallax_warning(result: &VisionResult) -> bool {
    result.warnings.iter().any(PhotoWarning::is_parallax)
}

#[test]
fn the_camera_is_found_from_the_paper_corners() -> TestResult {
    for (camera, height_share, foot_mm) in [(LEVEL, 0.01, 2.0), (TILTED, 0.01, 2.0)] {
        let shot = photo((&camera, &[disc((CENTRE, 52.0))]))?;
        let found = measure((&shot, PHOTO_WITH_LENS))?
            .camera
            .ok_or("no camera")?;
        assert_eq!(found.focal_source, FocalSource::Exif);
        let height_error = (found.height_mm - camera.height_mm).abs() / camera.height_mm;
        assert!(height_error <= height_share, "{camera:?}: found {found:?}");
        let foot_error =
            (found.foot_mm[0] - camera.foot_mm[0]).hypot(found.foot_mm[1] - camera.foot_mm[1]);
        assert!(foot_error <= foot_mm, "{camera:?}: found {found:?}");
    }
    Ok(())
}

#[test]
fn without_a_focal_length_a_wide_lens_is_assumed() -> TestResult {
    let shot = photo((&LEVEL, &[disc((CENTRE, 52.0))]))?;
    let run = Run {
        focal_35mm: None,
        ..PHOTO_WITH_LENS
    };
    let found = measure((&shot, run))?.camera.ok_or("no camera")?;
    assert_eq!(found.focal_source, FocalSource::Assumed);
    // Level, the camera's height scales with the lens: 24 mm puts it nearer than it was,
    // so the warning errs on the safe side.
    let expected = LEVEL.height_mm * 24.0 / 26.0;
    assert!(found.height_mm < LEVEL.height_mm);
    assert!(
        (found.height_mm - expected).abs() <= 0.03 * expected,
        "{found:?}"
    );
    Ok(())
}

/// An upright cylinder `height_mm` tall seen by `camera`: each 1 mm slice lands on the
/// paper's plane pushed away from the point below the lens, and bigger.
fn cylinder((camera, radius_mm, height_mm): (&Pinhole, f64, f64)) -> Vec<Drawing> {
    let lens = camera.height_mm;
    (0..=height_mm as u32)
        .map(|z| {
            let grow = lens / (lens - f64::from(z));
            let at = [0, 1]
                .map(|axis| camera.foot_mm[axis] + (CENTRE[axis] - camera.foot_mm[axis]) * grow);
            disc((at, 2.0 * radius_mm * grow))
        })
        .collect()
}

#[test]
fn the_warning_says_how_far_a_tall_cylinder_reaches() -> TestResult {
    // Level, the top just looks bigger; tilted, it slides away from the camera as well.
    for (camera, height_mm, expected) in [(LEVEL, 80.0, 7.7), (TILTED, 60.0, 51.9)] {
        let radius = 26.0;
        let shot = photo((&camera, &cylinder((&camera, radius, height_mm))))?;
        let run = Run {
            height_mm: Some(height_mm),
            ..PHOTO_WITH_LENS
        };
        let result = measure((&shot, run))?;
        let (warned_height, up_to_mm) = reach_warning(&result).ok_or("no warning")?;
        assert_eq!(warned_height, height_mm);
        assert_eq!(result.products[0].height_mm, Some(height_mm));

        // How far the measured outline really reaches past the cylinder's edge.
        let outline = paper_mm((A4, camera.quad(A4), &result.products[0].contour_px))?;
        let reach = outline
            .iter()
            .map(|[x, y]| (x - CENTRE[0]).hypot(y - CENTRE[1]) - radius)
            .fold(0.0, f64::max);
        let label = format!("{camera:?}: reach {reach:.2} mm, warned up to {up_to_mm} mm");
        assert!(reach <= up_to_mm + 0.3, "{label}");
        assert!(reach >= 0.9 * up_to_mm - 0.3, "the bound is loose: {label}");
        assert!((up_to_mm - expected).abs() <= 0.05 * expected, "{label}");
    }
    Ok(())
}

#[test]
fn a_low_product_gets_no_tall_warning() -> TestResult {
    let shot = photo((&LEVEL, &[disc((CENTRE, 52.0))]))?;
    // 1 mm tall reaches 1 / 350 of 26 mm: under the 0.5 mm bar.
    let low = measure((
        &shot,
        Run {
            height_mm: Some(1.0),
            ..PHOTO_WITH_LENS
        },
    ))?;
    assert!(!has_parallax_warning(&low), "{:?}", low.warnings);
    assert_eq!(low.products[0].height_mm, Some(1.0));

    // 30 mm reaches about 30 / 350 of 26 mm.
    let taller = measure((
        &shot,
        Run {
            height_mm: Some(30.0),
            ..PHOTO_WITH_LENS
        },
    ))?;
    let (_, up_to_mm) = reach_warning(&taller).ok_or("no warning")?;
    assert!((2.1..=2.5).contains(&up_to_mm), "{up_to_mm}");
    Ok(())
}

#[test]
fn set_height_matches_a_run_with_the_height_and_keeps_other_warnings() -> TestResult {
    let shot = photo((
        &LEVEL,
        &[disc(([105.0, 90.0], 60.0)), disc(([105.0, 220.0], 40.0))],
    ))?;
    let plain = measure((&shot, PHOTO_WITH_LENS))?;
    assert_eq!(plain.warnings[0], PhotoWarning::TallProductLooksBigger);
    let other_objects = PhotoWarning::OtherObjects { count: 1 };
    assert!(plain.warnings.contains(&other_objects));

    let with_height = set_height((&plain, 80.0))?;
    let run = Run {
        height_mm: Some(80.0),
        ..PHOTO_WITH_LENS
    };
    assert_eq!(with_height, measure((&shot, run))?);
    assert!(reach_warning(&with_height).is_some());
    assert!(with_height.warnings.contains(&other_objects));

    // Lowered, the tall-product warning goes; the rest stays.
    let lowered = set_height((&with_height, 1.0))?;
    assert!(!has_parallax_warning(&lowered));
    assert!(lowered.warnings.contains(&other_objects));
    assert_eq!(lowered.products[0].height_mm, Some(1.0));

    for height_mm in [0.0, -5.0, f64::NAN, 1500.0] {
        let error = set_height((&plain, height_mm)).unwrap_err();
        assert!(
            matches!(
                error,
                VisionError::InvalidInput {
                    field: "height_mm",
                    ..
                }
            ),
            "{height_mm}: {error}"
        );
    }
    Ok(())
}

#[test]
fn a_trace_gets_no_tall_warning() -> TestResult {
    let traced = Drawing::PenCircle {
        center_mm: (CENTRE[0], CENTRE[1]),
        diameter_mm: 60.0,
        pen_mm: 0.5,
    };
    let shot = photo((&LEVEL, &[traced]))?;
    let run = Run {
        mode: CaptureMode::Trace,
        height_mm: Some(120.0),
        ..PHOTO_WITH_LENS
    };
    let result = measure((&shot, run))?;
    assert!(result.warnings.is_empty(), "{:?}", result.warnings);
    assert_eq!(result.products[0].height_mm, Some(120.0));
    Ok(())
}

#[test]
fn an_edit_keeps_the_height_and_rechecks_the_warning() -> TestResult {
    let shot = photo((&LEVEL, &[disc((CENTRE, 52.0))]))?;
    let previous = set_height((&measure((&shot, PHOTO_WITH_LENS))?, 30.0))?;
    // A 40 mm square in the middle: its corners lie 28.3 mm from below the lens.
    let corners = [[85.0, 128.5], [125.0, 128.5], [125.0, 168.5], [85.0, 168.5]];
    let outline = photo_px((A4, LEVEL.quad(A4), &corners))?;
    let edited = accept_edit((&previous, &outline))?;
    assert_eq!(edited.products[0].height_mm, Some(30.0));
    assert_eq!(edited.camera, previous.camera);
    let (_, up_to_mm) = reach_warning(&edited).ok_or("no warning")?;
    assert!((2.3..=2.7).contains(&up_to_mm), "{up_to_mm}");
    Ok(())
}

#[test]
fn the_camera_and_height_are_in_the_result_and_the_inputs_are_checked() -> TestResult {
    let shot = photo((&LEVEL, &[disc((CENTRE, 52.0))]))?;
    let value = serde_json::to_value(measure((&shot, PHOTO_WITH_LENS))?)?;
    let camera = &value["camera"];
    assert!(camera["height_mm"].is_number());
    assert_eq!(camera["foot_mm"].as_array().map(Vec::len), Some(2));
    assert_eq!(camera["focal_source"], "exif");
    let back: VisionResult = serde_json::from_value(value)?;
    let tall = serde_json::to_value(set_height((&back, 80.0))?)?;
    assert_eq!(tall["products"][0]["height_mm"], 80.0);
    assert_eq!(tall["warnings"][0]["kind"], "outline_may_be_bigger");
    assert!(tall["warnings"][0]["up_to_mm"].is_number());

    let input: CaptureInput = serde_json::from_value(json!({
        "paper": "A4",
        "mode": "photo",
        "height_mm": 95,
        "focal_length_35mm": 26,
    }))?;
    assert_eq!(
        (input.height_mm, input.focal_length_35mm),
        (Some(95.0), Some(26.0))
    );
    let checks = [
        ("height_mm", (Some(0.0), None)),
        ("height_mm", (Some(f64::NAN), None)),
        ("height_mm", (Some(2000.0), None)),
        ("focal_length_35mm", (None, Some(0.0))),
        ("focal_length_35mm", (None, Some(f64::INFINITY))),
        ("focal_length_35mm", (None, Some(4000.0))),
    ];
    for (field, (height_mm, focal_length_35mm)) in checks {
        let bad = CaptureInput {
            height_mm,
            focal_length_35mm,
            ..input
        };
        // Checked before the file is even looked at.
        let error = vision::run((&bad, b"not an image")).unwrap_err();
        assert!(
            matches!(&error, VisionError::InvalidInput { field: named, .. } if *named == field),
            "{field}: {error}"
        );
    }
    Ok(())
}
