//! Generated photos with known sizes, shared by the pipeline tests: a top-down paper image
//! is drawn at 10 px/mm, then "photographed" with a perspective tilt onto a dark table.
#![allow(dead_code)]

use opencv::core::{self, Mat, Point, Point2d, Point2f, Scalar, Size, Vector};
use opencv::imgcodecs;
use opencv::imgproc;
use opencv::prelude::*;
use vision::camera::FULL_FRAME_DIAGONAL_MM;
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
pub type Quad = [(f32, f32); 4];

/// Upright in a landscape photo, top edge farther away (narrower).
pub const UPRIGHT_QUAD: Quad = [
    (1180.0, 200.0),
    (2860.0, 240.0),
    (2980.0, 2800.0),
    (1050.0, 2760.0),
];
/// Turned a quarter clockwise: the paper's short top edge runs down the right of the photo.
pub const SIDEWAYS_QUAD: Quad = [
    (3420.0, 380.0),
    (3500.0, 2520.0),
    (460.0, 2580.0),
    (520.0, 420.0),
];

/// A pinhole camera above the paper, looking at the paper's centre: photos whose camera
/// is known exactly (M1-C4). The paper's frame has x along its top edge, y down the sheet
/// and z into the table, so the lens sits at negative z.
#[derive(Debug, Clone, Copy)]
pub struct Pinhole {
    /// The point on the paper's plane straight below the lens, in mm.
    pub foot_mm: [f64; 2],
    /// The lens's height above the paper, in mm.
    pub height_mm: f64,
    /// Focal length in 35 mm terms, in mm.
    pub focal_35mm: f64,
}

impl Pinhole {
    /// Where the paper's own corners land in a `PHOTO_SIZE` photo, for `photograph`.
    pub fn quad(&self, paper: PaperSize) -> Quad {
        let (width, height) = paper.size_mm();
        [[0.0, 0.0], [width, 0.0], [width, height], [0.0, height]]
            .map(|corner| self.project((paper, corner)))
    }

    fn project(&self, (paper, [x, y]): (PaperSize, [f64; 2])) -> (f32, f32) {
        let (photo_width, photo_height) = (f64::from(PHOTO_SIZE.0), f64::from(PHOTO_SIZE.1));
        let focal_px = self.focal_35mm * photo_width.hypot(photo_height) / FULL_FRAME_DIAGONAL_MM;
        let (paper_width, paper_height) = paper.size_mm();
        let lens = [self.foot_mm[0], self.foot_mm[1], -self.height_mm];
        let forward = unit(minus([paper_width / 2.0, paper_height / 2.0, 0.0], lens));
        // The photo's x runs along the paper's top edge, its y down the sheet.
        let right = unit(minus([1.0, 0.0, 0.0], scaled((forward, forward[0]))));
        let down = cross(forward, right);
        let seen = minus([x, y, 0.0], lens);
        let depth = dot(forward, seen);
        (
            (focal_px * dot(right, seen) / depth + photo_width / 2.0) as f32,
            (focal_px * dot(down, seen) / depth + photo_height / 2.0) as f32,
        )
    }
}

fn minus(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn scaled((v, factor): ([f64; 3], f64)) -> [f64; 3] {
    v.map(|value| value * factor)
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn unit(v: [f64; 3]) -> [f64; 3] {
    scaled((v, 1.0 / dot(v, v).sqrt()))
}

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
    paper_image_with(paper, std::slice::from_ref(drawing))
}

/// The paper seen straight from above at 10 px/mm, with several drawings on it.
pub fn paper_image_with(paper: PaperSize, drawings: &[Drawing]) -> opencv::Result<Mat> {
    let mut image = Mat::new_size_with_default(paper.size_px(), core::CV_8UC3, bgr(PAPER_BGR))?;
    for drawing in drawings {
        draw((&mut image, drawing))?;
    }
    Ok(image)
}

fn draw((image, drawing): (&mut Mat, &Drawing)) -> opencv::Result<()> {
    match *drawing {
        Drawing::Disc {
            center_mm,
            diameter_mm,
        } => imgproc::circle(
            image,
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
                image,
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
            image,
            shifted(center_mm.0, center_mm.1),
            (diameter_mm / 2.0 * PX_PER_MM * SHIFT_SCALE).round() as i32,
            bgr(INK_BGR),
            (pen_mm * PX_PER_MM).round() as i32,
            imgproc::LINE_AA,
            SHIFT,
        )?,
    }
    Ok(())
}

/// The transform from the paper image (10 px/mm) to the photo, as `photograph` uses.
fn paper_to_photo((size, quad): (Size, Quad)) -> opencv::Result<Mat> {
    let (width, height) = (size.width as f32, size.height as f32);
    let source = Vector::<Point2f>::from_iter([
        Point2f::new(0.0, 0.0),
        Point2f::new(width, 0.0),
        Point2f::new(width, height),
        Point2f::new(0.0, height),
    ]);
    let target = Vector::<Point2f>::from_iter(quad.map(|(x, y)| Point2f::new(x, y)));
    imgproc::get_perspective_transform_def(&source, &target)
}

fn project((points, transform): (&[[f64; 2]], &Mat)) -> opencv::Result<Vec<[f64; 2]>> {
    let source = Vector::<Point2d>::from_iter(points.iter().map(|&[x, y]| Point2d::new(x, y)));
    let mut target = Vector::<Point2d>::new();
    core::perspective_transform(&source, &mut target, transform)?;
    Ok(target.iter().map(|point| [point.x, point.y]).collect())
}

/// Where points on the paper, in mm, appear in a photo taken at `quad`: the true
/// positions, not the pipeline's.
pub fn photo_px(
    (paper, quad, points_mm): (PaperSize, Quad, &[[f64; 2]]),
) -> opencv::Result<Vec<[f64; 2]>> {
    let paper_px: Vec<[f64; 2]> = points_mm
        .iter()
        .map(|point| point.map(|value| value * PX_PER_MM))
        .collect();
    project((&paper_px, &paper_to_photo((paper.size_px(), quad))?))
}

/// Where photo points lie on the paper, in mm: the inverse of [`photo_px`].
pub fn paper_mm(
    (paper, quad, points_px): (PaperSize, Quad, &[[f64; 2]]),
) -> opencv::Result<Vec<[f64; 2]>> {
    let transform = paper_to_photo((paper.size_px(), quad))?
        .inv_def()?
        .to_mat()?;
    Ok(project((points_px, &transform))?
        .into_iter()
        .map(|point| point.map(|value| value / PX_PER_MM))
        .collect())
}

/// The image encoded as `extension` (".jpg", ".png", ".webp"), at high quality.
pub fn encode((image, extension): (&Mat, &str)) -> opencv::Result<Vec<u8>> {
    let params = match extension {
        ".jpg" => Vector::from_slice(&[imgcodecs::IMWRITE_JPEG_QUALITY, 95]),
        ".webp" => Vector::from_slice(&[imgcodecs::IMWRITE_WEBP_QUALITY, 95]),
        _ => Vector::new(),
    };
    let mut bytes = Vector::<u8>::new();
    imgcodecs::imencode(extension, image, &mut bytes, &params)?;
    Ok(bytes.to_vec())
}

/// Places the paper image onto a table photo with a perspective transform.
pub fn photograph(paper_image: &Mat, quad: Quad) -> opencv::Result<Mat> {
    let transform = paper_to_photo((paper_image.size()?, quad))?;
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
