//! The product's height and the tall-product warning (CLAUDE.md §5.2 fix B, DEC-09, M1-C4).
//!
//! From above, every slice of a product is seen from the lens, so a slice `z` mm up lands
//! on the paper's plane pushed away from the point below the lens, by `z ÷ (D − z)` of its
//! distance, with `D` the lens's height above the paper. The top pushes furthest. A point of
//! the measured outline at distance `ρ` from the point below the lens therefore lies at most
//! `ρ · h ÷ D` past the product's real edge, for a product `h` mm tall. The warning reports
//! the most of that over the outline: exact for an upright cylinder, an upper bound for
//! anything narrower at the top.

use crate::error::VisionError;
use crate::input;
use crate::paper::{PX_PER_MM, PaperMapping};
use crate::result::{CaptureMode, PhotoWarning, VisionResult, round2};

/// Below this, the top's push is lost in the measuring itself (§5.6: a median of 0.8 mm
/// on a size, so about 0.4 mm on each side), and no warning is given.
pub const PARALLAX_WARN_MM: f64 = 0.5;

/// The capture's result with the product's height, typed by the user (DEC-09), and the
/// tall-product warning worked out for it. Needs no photo.
pub fn set_height(
    (previous, height_mm): (&VisionResult, f64),
) -> Result<VisionResult, VisionError> {
    input::check_height(height_mm)?;
    let mut result = previous.clone();
    for product in &mut result.products {
        product.height_mm = Some(round2(height_mm));
    }
    update_parallax_warning(&mut result)?;
    Ok(result)
}

/// Puts the result's tall-product warning right for its mode, camera, product height and
/// outline: the old one goes, the right one (if any) comes first.
pub(crate) fn update_parallax_warning(result: &mut VisionResult) -> Result<(), VisionError> {
    result.warnings.retain(|warning| !warning.is_parallax());
    if let Some(warning) = parallax_warning(result)? {
        result.warnings.insert(0, warning);
    }
    Ok(())
}

/// The MVP's single product. A trace lies on the paper, so it has none.
fn parallax_warning(result: &VisionResult) -> Result<Option<PhotoWarning>, VisionError> {
    if result.mode == CaptureMode::Trace {
        return Ok(None);
    }
    let Some(product) = result.products.first() else {
        return Ok(None);
    };
    let (Some(height_mm), Some(camera)) = (product.height_mm, result.camera) else {
        return Ok(Some(PhotoWarning::TallProductLooksBigger));
    };
    // A lens no higher than the product means the camera estimate is off: say it plainly.
    if height_mm >= camera.height_mm {
        return Ok(Some(PhotoWarning::TallProductLooksBigger));
    }
    let mapping = PaperMapping::from_result(&result.paper)?;
    let [foot_x, foot_y] = camera.foot_mm;
    let farthest_mm = mapping
        .to_paper_px(&product.contour_px)?
        .iter()
        .map(|[x, y]| (x / PX_PER_MM - foot_x).hypot(y / PX_PER_MM - foot_y))
        .fold(0.0, f64::max);
    let up_to_mm = farthest_mm * height_mm / camera.height_mm;
    if up_to_mm < PARALLAX_WARN_MM {
        return Ok(None);
    }
    Ok(Some(PhotoWarning::OutlineMayBeBigger {
        height_mm,
        // Rounded up: it's a bound.
        up_to_mm: (up_to_mm * 10.0).ceil() / 10.0,
    }))
}
