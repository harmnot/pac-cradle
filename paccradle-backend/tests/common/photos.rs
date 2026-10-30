//! Photos vision can measure (M1-D5), drawn like the `vision` crate's own tests do: an A4
//! sheet at 10 px/mm with a product on it, "photographed" at a tilt onto a dark table, and
//! encoded as a JPEG. Positions on the sheet, in mm, map to the photo's pixels for taps and
//! edited outlines.

use opencv::core::{self, Mat, Point, Point2d, Point2f, Scalar, Size, Vector};
use opencv::imgcodecs;
use opencv::imgproc;
use opencv::prelude::*;

const PX_PER_MM: f64 = 10.0;
const A4_PX: (i32, i32) = (2100, 2970);
const PHOTO_SIZE: (i32, i32) = (4000, 3000);
/// Where the sheet's top-left, top-right, bottom-right and bottom-left corners land in the
/// photo: upright, its top edge farther away.
const QUAD: [(f32, f32); 4] = [
    (1180.0, 200.0),
    (2860.0, 240.0),
    (2980.0, 2800.0),
    (1050.0, 2760.0),
];
const PAPER_BGR: Scalar = Scalar::new(232.0, 236.0, 238.0, 0.0);
const TABLE_BGR: Scalar = Scalar::new(52.0, 58.0, 66.0, 0.0);
const PRODUCT_BGR: Scalar = Scalar::new(40.0, 90.0, 170.0, 0.0);

/// Where the disc sits on the sheet, in mm, and how wide it is.
pub const DISC_CENTER_MM: [f64; 2] = [105.0, 150.0];
pub const DISC_DIAMETER_MM: f64 = 52.0;

/// A 52 mm disc in the middle of an A4 sheet, as a 4000 × 3000 JPEG.
pub fn disc_photo() -> Vec<u8> {
    let mut sheet = blank_sheet();
    let center = Point::new(
        (DISC_CENTER_MM[0] * PX_PER_MM) as i32,
        (DISC_CENTER_MM[1] * PX_PER_MM) as i32,
    );
    let radius = (DISC_DIAMETER_MM / 2.0 * PX_PER_MM) as i32;
    imgproc::circle(
        &mut sheet,
        center,
        radius,
        PRODUCT_BGR,
        imgproc::FILLED,
        imgproc::LINE_AA,
        0,
    )
    .unwrap();
    jpeg(&photograph(&sheet))
}

/// The table alone: no sheet to find.
pub fn table_photo() -> Vec<u8> {
    let table = Mat::new_size_with_default(
        Size::new(PHOTO_SIZE.0, PHOTO_SIZE.1),
        core::CV_8UC3,
        TABLE_BGR,
    )
    .unwrap();
    jpeg(&table)
}

/// Where points on the sheet, in mm, appear in the photo, in pixels.
pub fn photo_px(points_mm: &[[f64; 2]]) -> Vec<[f64; 2]> {
    let source = Vector::<Point2d>::from_iter(
        points_mm
            .iter()
            .map(|&[x, y]| Point2d::new(x * PX_PER_MM, y * PX_PER_MM)),
    );
    let mut target = Vector::<Point2d>::new();
    core::perspective_transform(&source, &mut target, &sheet_to_photo()).unwrap();
    target.iter().map(|point| [point.x, point.y]).collect()
}

fn blank_sheet() -> Mat {
    Mat::new_size_with_default(Size::new(A4_PX.0, A4_PX.1), core::CV_8UC3, PAPER_BGR).unwrap()
}

fn sheet_to_photo() -> Mat {
    let (width, height) = (A4_PX.0 as f32, A4_PX.1 as f32);
    let source = Vector::<Point2f>::from_iter([
        Point2f::new(0.0, 0.0),
        Point2f::new(width, 0.0),
        Point2f::new(width, height),
        Point2f::new(0.0, height),
    ]);
    let target = Vector::<Point2f>::from_iter(QUAD.map(|(x, y)| Point2f::new(x, y)));
    imgproc::get_perspective_transform_def(&source, &target).unwrap()
}

fn photograph(sheet: &Mat) -> Mat {
    let size = Size::new(PHOTO_SIZE.0, PHOTO_SIZE.1);
    let mut photo = Mat::new_size_with_default(size, core::CV_8UC3, TABLE_BGR).unwrap();
    imgproc::warp_perspective(
        sheet,
        &mut photo,
        &sheet_to_photo(),
        size,
        imgproc::INTER_LINEAR,
        core::BORDER_TRANSPARENT,
        Scalar::default(),
    )
    .unwrap();
    photo
}

fn jpeg(image: &Mat) -> Vec<u8> {
    let params = Vector::from_slice(&[imgcodecs::IMWRITE_JPEG_QUALITY, 95]);
    let mut bytes = Vector::<u8>::new();
    imgcodecs::imencode(".jpg", image, &mut bytes, &params).unwrap();
    bytes.to_vec()
}
