//! Where the camera was, from the paper's 4 corners and the lens's focal length (M1-C4).
//!
//! The paper is a known rectangle on the table, so its corners in the photo fix the camera's
//! position once the lens is known: the photo maps the paper's plane by
//! `K · [r1 r2 t]`, with `K` the lens (focal length, centre of the photo) and `r1`, `r2`, `t`
//! the paper's axes and corner as the camera sees them. The tall-product warning uses the
//! camera's height above the paper and the point straight below it. Pose correction
//! (M3-03) can build on the same numbers.

use opencv::core::Size;

use crate::error::VisionError;
use crate::paper::PaperMapping;
use crate::result::{CameraEstimate, FocalSource, round2};

/// The film diagonal 35 mm-equivalent focal lengths are measured against.
pub const FULL_FRAME_DIAGONAL_MM: f64 = 43.27;
/// The lens assumed when the photo's EXIF gives none: a wide phone main camera. Wider than
/// most, so the camera comes out nearer than it was and the warning's bound stays an upper
/// bound, unless the ultra-wide lens was used.
pub const ASSUMED_FOCAL_LENGTH_35MM: f64 = 24.0;

type Vec3 = [f64; 3];

/// Works out the camera from the paper's mapping, the photo's size and the lens's focal
/// length in 35 mm terms (`None`: assume [`ASSUMED_FOCAL_LENGTH_35MM`]). Assumes square
/// pixels and the lens centred on the photo, true of phone photos that weren't cropped.
/// `None` when the corners don't give a camera above the paper.
pub fn estimate(
    (mapping, photo, focal_length_35mm): (&PaperMapping, Size, Option<f64>),
) -> Result<Option<CameraEstimate>, VisionError> {
    let (focal_35mm, focal_source) = match focal_length_35mm {
        Some(focal) => (focal, FocalSource::Exif),
        None => (ASSUMED_FOCAL_LENGTH_35MM, FocalSource::Assumed),
    };
    let (width, height) = (f64::from(photo.width), f64::from(photo.height));
    let focal_px = focal_35mm * width.hypot(height) / FULL_FRAME_DIAGONAL_MM;
    let (centre_x, centre_y) = (width / 2.0, height / 2.0);
    let paper_to_photo = mapping.photo_from_paper_mm()?;

    // K⁻¹ times each column: the paper's x and y axes and its corner, seen from the camera,
    // all up to one unknown scale.
    let seen = |column: usize| -> Vec3 {
        let [x, y, w] = paper_to_photo.map(|row| row[column]);
        [
            (x - centre_x * w) / focal_px,
            (y - centre_y * w) / focal_px,
            w,
        ]
    };
    let (x_axis, y_axis, corner) = (seen(0), seen(1), seen(2));
    // The axes are a millimetre long, and the paper lies in front of the camera.
    let mut scale = 2.0 / (length(x_axis) + length(y_axis));
    if corner[2] < 0.0 {
        scale = -scale;
    }
    let (r1, r2, t) = (
        times(x_axis, scale),
        times(y_axis, scale),
        times(corner, scale),
    );
    let r3 = cross(r1, r2);

    // The lens in the paper's frame (x along the top edge, y down the sheet, z into the
    // table) is −Rᵀ·t; it sits on the table's other side, at negative z.
    let lens = [-dot(r1, t), -dot(r2, t), -dot(r3, t)];
    let height_mm = -lens[2];
    if !(lens.iter().all(|value| value.is_finite()) && height_mm > 0.0) {
        return Ok(None);
    }
    Ok(Some(CameraEstimate {
        height_mm: round2(height_mm),
        foot_mm: [round2(lens[0]), round2(lens[1])],
        focal_source,
    }))
}

fn length(v: Vec3) -> f64 {
    dot(v, v).sqrt()
}

fn times(v: Vec3, factor: f64) -> Vec3 {
    v.map(|value| value * factor)
}

fn dot(a: Vec3, b: Vec3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: Vec3, b: Vec3) -> Vec3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
