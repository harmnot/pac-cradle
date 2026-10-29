//! Cut the product (photo mode) or the traced outline (trace mode) out of the
//! flattened paper (CLAUDE.md §5.2 fix A, §5.3 steps 4–5).

use opencv::core::{self, Mat, Point, Point2f, Rect, Scalar, Size, Vec4i, Vector};
use opencv::imgproc;
use opencv::prelude::*;

use crate::error::VisionError;
use crate::paper::{PX_PER_MM, mm_to_px, to_gray};
use crate::result::CaptureMode;

/// Band at the paper edge that is ignored (shadows, paper curl, cut edge).
pub const EDGE_IGNORE_MM: f64 = 3.0;
/// The paper colour is sampled between the ignored band and this distance from the edge,
/// where the capture rules keep the product away (CLAUDE.md §5.1: at least 2 cm).
const PAPER_SAMPLE_BAND_MM: f64 = 15.0;
/// Shadows mostly change lightness, so lightness counts half in the colour distance.
const LIGHTNESS_WEIGHT: f64 = 0.5;
/// Lowest colour distance (Lab units) that counts as "not paper", even if Otsu picks less.
const MIN_COLOR_DISTANCE: f64 = 12.0;
/// Smallest product or traced outline accepted, in mm².
pub const MIN_OBJECT_AREA_MM2: f64 = 25.0;
/// Other blobs larger than this share of the product count as extra objects.
const EXTRA_BLOB_SHARE: f64 = 0.05;
/// Mask clean-up kernels (pixels at 10 px/mm).
const OPEN_KERNEL_PX: i32 = 5;
const CLOSE_KERNEL_PX: i32 = 15;
/// GrabCut: uncertain band on each side of the first mask, and iterations.
const GRABCUT_BAND_PX: i32 = 25;
const GRABCUT_ITERATIONS: i32 = 3;
/// Trace mode: local-threshold block (≈ 5 mm) and how much darker than its surroundings ink is.
const TRACE_BLOCK_PX: i32 = 51;
const TRACE_DARKER_BY: f64 = 15.0;
const TRACE_CLOSE_KERNEL_PX: i32 = 5;
/// How far outside an outline a tap may land and still pick it: about a fingertip's miss
/// on a phone showing the whole sheet.
const TAP_REACH_MM: f64 = 10.0;

/// Which outline to keep when there are several.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Pick {
    /// The biggest. The others big enough to matter count as extra objects.
    Largest,
    /// The one under the user's tap, in flattened-paper pixels, or failing that the nearest
    /// within 10 mm of it (CLAUDE.md §3 step 4). The others don't count: the user picked.
    At(Point2f),
}

impl Pick {
    /// The error when nothing fits: `otherwise` for the largest, nothing at the tap for a tap.
    fn missing(self, otherwise: VisionError) -> VisionError {
        match self {
            Pick::Largest => otherwise,
            Pick::At(_) => VisionError::NothingAtTap,
        }
    }
}

/// How to cut the product out of the flattened paper.
#[derive(Debug, Clone, Copy)]
pub struct Settings {
    pub mode: CaptureMode,
    pub pick: Pick,
    /// Photo mode: refine the first mask with GrabCut.
    pub refine_with_grabcut: bool,
    /// Trace mode: the pen line's width, in mm.
    pub pen_width_mm: f64,
}

/// A product mask on the flattened paper (255 = product) and how many other objects were seen.
pub struct Segmented {
    /// 8-bit mask, same size as the flattened paper.
    pub mask: Mat,
    /// Other objects or outlines big enough to matter (lowers confidence).
    pub extra_blobs: usize,
}

/// Cuts the product (photo mode) or the traced outline (trace mode) out of the flattened
/// paper.
pub fn cut_out(flat: &Mat, settings: &Settings) -> Result<Segmented, VisionError> {
    match settings.mode {
        CaptureMode::Photo => product_mask(flat, settings),
        CaptureMode::Trace => trace_mask(flat, settings),
    }
}

/// Photo mode: pixels whose colour differs from the paper, cleaned up and refined with GrabCut.
fn product_mask(flat: &Mat, settings: &Settings) -> Result<Segmented, VisionError> {
    let distance = color_distance(flat)?;
    let mut foreground = Mat::default();
    let otsu = imgproc::threshold(
        &distance,
        &mut foreground,
        0.0,
        255.0,
        imgproc::THRESH_BINARY | imgproc::THRESH_OTSU,
    )?;
    if otsu < MIN_COLOR_DISTANCE {
        imgproc::threshold(
            &distance,
            &mut foreground,
            MIN_COLOR_DISTANCE,
            255.0,
            imgproc::THRESH_BINARY,
        )?;
    }
    let inside = keep_inside_edge_band(&foreground)?;
    let opened = morph(&inside, imgproc::MORPH_OPEN, OPEN_KERNEL_PX)?;
    let cleaned = morph(&opened, imgproc::MORPH_CLOSE, CLOSE_KERNEL_PX)?;

    let missing = || settings.pick.missing(VisionError::ProductNotFound);
    let first = blob(&cleaned, settings.pick)?.ok_or_else(missing)?;
    if !settings.refine_with_grabcut {
        return Ok(first);
    }
    let refined = refine_with_grabcut(flat, &first.mask)?;
    let product = blob(&refined, settings.pick)?.ok_or_else(missing)?;
    Ok(Segmented {
        mask: product.mask,
        extra_blobs: first.extra_blobs,
    })
}

/// Trace mode: finds the closed pen line, takes its inner edge, then adds back half the pen
/// width so the outline matches the product edge (CLAUDE.md §5.2 fix A).
fn trace_mask(flat: &Mat, settings: &Settings) -> Result<Segmented, VisionError> {
    let mut ink = Mat::default();
    imgproc::adaptive_threshold(
        &to_gray(flat)?,
        &mut ink,
        255.0,
        imgproc::ADAPTIVE_THRESH_GAUSSIAN_C,
        imgproc::THRESH_BINARY_INV,
        TRACE_BLOCK_PX,
        TRACE_DARKER_BY,
    )?;
    let line = morph(
        &keep_inside_edge_band(&ink)?,
        imgproc::MORPH_CLOSE,
        TRACE_CLOSE_KERNEL_PX,
    )?;

    let mut contours = Vector::<Vector<Point>>::new();
    let mut hierarchy = Vector::<Vec4i>::new();
    imgproc::find_contours_with_hierarchy_def(
        &line,
        &mut contours,
        &mut hierarchy,
        imgproc::RETR_CCOMP,
        imgproc::CHAIN_APPROX_NONE,
    )?;
    // In RETR_CCOMP, holes are the contours that have a parent: the inside of each pen loop.
    let mut holes = Vec::new();
    for (index, contour) in contours.iter().enumerate() {
        if hierarchy.get(index)?[3] >= 0 {
            holes.push((index, imgproc::contour_area_def(&contour)?));
        }
    }
    let loops = Found {
        contours,
        candidates: holes,
    };
    let inside = loops
        .choose(settings.pick)?
        .ok_or_else(|| settings.pick.missing(VisionError::TraceNotFound))?;

    // The filled hole contour includes one ring of ink pixels; erode it back to the hole itself.
    let filled = fill_contour(&loops.contours, inside.index, line.size()?)?;
    let cross = imgproc::get_structuring_element_def(imgproc::MORPH_CROSS, Size::new(3, 3))?;
    let mut inner = Mat::default();
    imgproc::erode_def(&filled, &mut inner, &cross)?;
    let mask = grow(&inner, settings.pen_width_mm / 2.0 * PX_PER_MM)?;
    Ok(Segmented {
        mask,
        extra_blobs: inside.extra_blobs,
    })
}

/// Per-pixel colour distance from the paper (Lab, lightness weighted down), as 8-bit.
fn color_distance(flat: &Mat) -> Result<Mat, VisionError> {
    let mut lab = Mat::default();
    imgproc::cvt_color_def(flat, &mut lab, imgproc::COLOR_BGR2Lab)?;
    let paper = core::mean(&lab, &paper_sample_mask(flat.size()?)?)?;

    let mut channels = Vector::<Mat>::new();
    core::split(&lab, &mut channels)?;
    let weights = [LIGHTNESS_WEIGHT, 1.0, 1.0];
    let mut sum = Mat::default();
    for (index, weight) in weights.iter().enumerate() {
        let mut difference = Mat::default();
        channels.get(index)?.convert_to(
            &mut difference,
            core::CV_32F,
            *weight,
            -weight * paper[index],
        )?;
        let mut squared = Mat::default();
        core::multiply_def(&difference, &difference, &mut squared)?;
        if index == 0 {
            sum = squared;
        } else {
            let mut total = Mat::default();
            core::add_def(&sum, &squared, &mut total)?;
            sum = total;
        }
    }
    let mut distance = Mat::default();
    core::sqrt(&sum, &mut distance)?;
    let mut distance_8bit = Mat::default();
    distance.convert_to(&mut distance_8bit, core::CV_8U, 1.0, 0.0)?;
    let mut smoothed = Mat::default();
    imgproc::gaussian_blur_def(&distance_8bit, &mut smoothed, Size::new(3, 3), 0.0)?;
    Ok(smoothed)
}

/// Mask of the band where the paper colour is sampled (3–15 mm from the edges).
fn paper_sample_mask(size: Size) -> Result<Mat, VisionError> {
    let mut mask = rect_mask(size, mm_to_px(EDGE_IGNORE_MM))?;
    let inner = inset_rect(size, mm_to_px(PAPER_SAMPLE_BAND_MM));
    imgproc::rectangle(
        &mut mask,
        inner,
        Scalar::all(0.0),
        imgproc::FILLED,
        imgproc::LINE_8,
        0,
    )?;
    Ok(mask)
}

/// Clears everything in the ignored band at the paper edge.
fn keep_inside_edge_band(mask: &Mat) -> Result<Mat, VisionError> {
    let inside = rect_mask(mask.size()?, mm_to_px(EDGE_IGNORE_MM))?;
    let mut kept = Mat::default();
    core::bitwise_and_def(mask, &inside, &mut kept)?;
    Ok(kept)
}

fn rect_mask(size: Size, inset_px: i32) -> Result<Mat, VisionError> {
    let mut mask = Mat::zeros_size(size, core::CV_8UC1)?.to_mat()?;
    imgproc::rectangle(
        &mut mask,
        inset_rect(size, inset_px),
        Scalar::all(255.0),
        imgproc::FILLED,
        imgproc::LINE_8,
        0,
    )?;
    Ok(mask)
}

fn inset_rect(size: Size, inset_px: i32) -> Rect {
    Rect::new(
        inset_px,
        inset_px,
        (size.width - 2 * inset_px).max(0),
        (size.height - 2 * inset_px).max(0),
    )
}

fn morph(mask: &Mat, operation: i32, kernel_px: i32) -> Result<Mat, VisionError> {
    let kernel = imgproc::get_structuring_element_def(
        imgproc::MORPH_ELLIPSE,
        Size::new(kernel_px, kernel_px),
    )?;
    let mut result = Mat::default();
    imgproc::morphology_ex_def(mask, &mut result, operation, &kernel)?;
    Ok(result)
}

fn min_area_px() -> f64 {
    MIN_OBJECT_AREA_MM2 * PX_PER_MM * PX_PER_MM
}

fn pick_largest(areas: &[(usize, f64)]) -> Option<(usize, f64)> {
    areas.iter().copied().max_by(|a, b| a.1.total_cmp(&b.1))
}

/// Keeps only the blob `pick` chooses.
fn blob(mask: &Mat, pick: Pick) -> Result<Option<Segmented>, VisionError> {
    let mut contours = Vector::<Vector<Point>>::new();
    imgproc::find_contours_def(
        mask,
        &mut contours,
        imgproc::RETR_EXTERNAL,
        imgproc::CHAIN_APPROX_NONE,
    )?;
    let mut candidates = Vec::new();
    for (index, contour) in contours.iter().enumerate() {
        candidates.push((index, imgproc::contour_area_def(&contour)?));
    }
    let blobs = Found {
        contours,
        candidates,
    };
    let Some(chosen) = blobs.choose(pick)? else {
        return Ok(None);
    };
    Ok(Some(Segmented {
        mask: fill_contour(&blobs.contours, chosen.index, mask.size()?)?,
        extra_blobs: chosen.extra_blobs,
    }))
}

/// Outlines found in a mask, and the ones that may be the product with their areas in px.
struct Found {
    contours: Vector<Vector<Point>>,
    candidates: Vec<(usize, f64)>,
}

/// The outline kept, and how many other objects count against it.
struct Chosen {
    index: usize,
    extra_blobs: usize,
}

impl Found {
    fn choose(&self, pick: Pick) -> Result<Option<Chosen>, VisionError> {
        match pick {
            Pick::Largest => Ok(self.largest()),
            Pick::At(tap) => self.at(tap),
        }
    }

    /// The biggest candidate, if it's big enough; others over 5 % of its area count as
    /// extra objects.
    fn largest(&self) -> Option<Chosen> {
        let (index, area) = pick_largest(&self.candidates)?;
        if area < min_area_px() {
            return None;
        }
        let extra_blobs = self
            .candidates
            .iter()
            .filter(|(other, other_area)| *other != index && *other_area > EXTRA_BLOB_SHARE * area)
            .count();
        Some(Chosen { index, extra_blobs })
    }

    /// The smallest big-enough candidate holding the tap (a pen loop inside another is the
    /// nearer one); failing that, the nearest within reach.
    fn at(&self, tap: Point2f) -> Result<Option<Chosen>, VisionError> {
        let reach_px = TAP_REACH_MM * PX_PER_MM;
        let mut holding: Option<(usize, f64)> = None;
        let mut nearest: Option<(usize, f64)> = None;
        for &(index, area) in &self.candidates {
            if area < min_area_px() {
                continue;
            }
            // Distance to the outline in px: positive inside, negative outside.
            let distance = imgproc::point_polygon_test(&self.contours.get(index)?, tap, true)?;
            if distance >= 0.0 {
                if holding.is_none_or(|(_, smallest)| area < smallest) {
                    holding = Some((index, area));
                }
            } else if distance >= -reach_px && nearest.is_none_or(|(_, best)| distance > best) {
                nearest = Some((index, distance));
            }
        }
        Ok(holding.or(nearest).map(|(index, _)| Chosen {
            index,
            extra_blobs: 0,
        }))
    }
}

fn fill_contour(
    contours: &Vector<Vector<Point>>,
    index: usize,
    size: Size,
) -> Result<Mat, VisionError> {
    let mut mask = Mat::zeros_size(size, core::CV_8UC1)?.to_mat()?;
    imgproc::draw_contours(
        &mut mask,
        contours,
        index as i32,
        Scalar::all(255.0),
        imgproc::FILLED,
        imgproc::LINE_8,
        &core::no_array(),
        i32::MAX,
        Point::default(),
    )?;
    Ok(mask)
}

/// Grows a mask outward by `radius_px` (Euclidean), e.g. to add back half the pen width.
fn grow(mask: &Mat, radius_px: f64) -> Result<Mat, VisionError> {
    let mut outside = Mat::default();
    core::bitwise_not_def(mask, &mut outside)?;
    let mut distance = Mat::default();
    imgproc::distance_transform_def(
        &outside,
        &mut distance,
        imgproc::DIST_L2,
        imgproc::DIST_MASK_PRECISE,
    )?;
    let mut within = Mat::default();
    imgproc::threshold(
        &distance,
        &mut within,
        radius_px,
        255.0,
        imgproc::THRESH_BINARY_INV,
    )?;
    let mut grown = Mat::default();
    within.convert_to(&mut grown, core::CV_8U, 1.0, 0.0)?;
    Ok(grown)
}

/// Refines a rough mask with GrabCut around it: a sure core, an uncertain band, sure background.
fn refine_with_grabcut(flat: &Mat, rough: &Mat) -> Result<Mat, VisionError> {
    let bounds = imgproc::bounding_rect(rough)?;
    let margin = 2 * GRABCUT_BAND_PX;
    let image_rect = Rect::new(0, 0, flat.cols(), flat.rows());
    let roi = Rect::new(
        bounds.x - margin,
        bounds.y - margin,
        bounds.width + 2 * margin,
        bounds.height + 2 * margin,
    ) & image_rect;

    let image = Mat::roi(flat, roi)?.try_clone()?;
    let rough_roi = Mat::roi(rough, roi)?.try_clone()?;
    let band = imgproc::get_structuring_element_def(
        imgproc::MORPH_ELLIPSE,
        Size::new(2 * GRABCUT_BAND_PX + 1, 2 * GRABCUT_BAND_PX + 1),
    )?;
    let mut dilated = Mat::default();
    imgproc::dilate_def(&rough_roi, &mut dilated, &band)?;
    let mut eroded = Mat::default();
    imgproc::erode_def(&rough_roi, &mut eroded, &band)?;

    let mut labels = Mat::new_size_with_default(
        roi.size(),
        core::CV_8UC1,
        Scalar::all(f64::from(imgproc::GC_BGD)),
    )?;
    labels.set_to(&Scalar::all(f64::from(imgproc::GC_PR_BGD)), &dilated)?;
    labels.set_to(&Scalar::all(f64::from(imgproc::GC_PR_FGD)), &rough_roi)?;
    labels.set_to(&Scalar::all(f64::from(imgproc::GC_FGD)), &eroded)?;
    let mut background_model = Mat::default();
    let mut foreground_model = Mat::default();
    imgproc::grab_cut(
        &image,
        &mut labels,
        Rect::default(),
        &mut background_model,
        &mut foreground_model,
        GRABCUT_ITERATIONS,
        imgproc::GC_INIT_WITH_MASK,
    )?;

    // GC_FGD = 1 and GC_PR_FGD = 3: foreground labels have the lowest bit set.
    let mut lowest_bit = Mat::default();
    core::bitwise_and_def(&labels, &Scalar::all(1.0), &mut lowest_bit)?;
    let mut foreground = Mat::default();
    imgproc::threshold(
        &lowest_bit,
        &mut foreground,
        0.0,
        255.0,
        imgproc::THRESH_BINARY,
    )?;

    let mut full = Mat::zeros_size(flat.size()?, core::CV_8UC1)?.to_mat()?;
    let mut target = Mat::roi_mut(&mut full, roi)?;
    foreground.copy_to(&mut target)?;
    Ok(full)
}
