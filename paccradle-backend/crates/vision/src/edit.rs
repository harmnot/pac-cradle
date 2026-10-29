//! Outlines the user edited (CLAUDE.md §3 step 4, M1-C3): measured exactly as drawn, with
//! no detection and no photo.

use crate::error::VisionError;
use crate::paper::{PX_PER_MM, PaperMapping};
use crate::result::{ContourSource, VisionResult};
use crate::segment::MIN_OBJECT_AREA_MM2;
use crate::{ProductParts, height, measure, product_result};

/// Most points an edited outline may have. Detected outlines have tens to a few hundred;
/// the cap keeps the crossing check to a few milliseconds.
pub const MAX_EDIT_POINTS: usize = 2000;
const FIELD: &str = "edited_contour_px";
const POINTS_ALLOWED: &str = "3 to 2000 points";
const AREA_ALLOWED: &str = "at least 25 mm²";

type Point = [f64; 2];
type Edge = (Point, Point);

/// Measures an outline the user edited, exactly as drawn, in place of the capture's
/// product.
///
/// `previous` is the capture's stored result: it gives the paper, the mode and the level.
/// `outline_px` is the outline's corners in photo pixels (the frame of `contour_px`), in
/// order round the product. Repeated points add nothing and are dropped, as is a last
/// point repeating the first. The photo isn't needed, and nothing is detected or
/// simplified.
///
/// The product gets `contour_source: user` and confidence 1, and keeps its height. Only the
/// tall-product warning stays (photo mode), worked out again for the new outline: the
/// others are about finding the product, which the user has now done.
pub fn accept_edit(
    (previous, outline_px): (&VisionResult, &[Point]),
) -> Result<VisionResult, VisionError> {
    if !(3..=MAX_EDIT_POINTS).contains(&outline_px.len()) {
        return Err(invalid((
            format!("{} points", outline_px.len()),
            POINTS_ALLOWED,
        )));
    }
    if !outline_px.iter().flatten().all(|value| value.is_finite()) {
        return Err(invalid((
            "a point that isn't a number".to_owned(),
            "pixel positions",
        )));
    }
    let photo_points = without_repeats(outline_px);
    let mapping = PaperMapping::from_result(&previous.paper)?;
    let paper_points = mapping.to_paper_px(&photo_points)?;
    if !paper_points.iter().all(|&point| mapping.on_paper(point)) {
        return Err(invalid((
            "a point off the paper".to_owned(),
            "points on the paper",
        )));
    }
    // First, as a crossing outline's area means nothing: a bow tie's halves cancel out.
    if crosses_itself(&paper_points) {
        return Err(VisionError::OutlineCrossesItself);
    }
    let area_mm2 = signed_area(&paper_points).abs() / (PX_PER_MM * PX_PER_MM);
    if area_mm2 < MIN_OBJECT_AREA_MM2 {
        return Err(invalid((
            format!("an outline of {area_mm2:.1} mm²"),
            AREA_ALLOWED,
        )));
    }

    let outline = measure::measure_points(&paper_points)?;
    let mut result = VisionResult {
        mode: previous.mode,
        paper: previous.paper.clone(),
        camera: previous.camera,
        products: vec![product_result(ProductParts {
            outline: &outline,
            contour_px: photo_points,
            confidence: 1.0,
            source: ContourSource::User,
            height_mm: previous
                .products
                .first()
                .and_then(|product| product.height_mm),
        })],
        warnings: Vec::new(),
        level_used: previous.level_used,
    };
    height::update_parallax_warning(&mut result)?;
    Ok(result)
}

fn invalid((value, allowed): (String, &'static str)) -> VisionError {
    VisionError::InvalidInput {
        field: FIELD,
        value,
        allowed,
    }
}

/// The points without repeats in a row, nor a last point repeating the first.
fn without_repeats(points: &[Point]) -> Vec<Point> {
    let mut kept: Vec<Point> = Vec::with_capacity(points.len());
    for &point in points {
        if kept.last() != Some(&point) {
            kept.push(point);
        }
    }
    if kept.len() > 1 && kept.first() == kept.last() {
        kept.pop();
    }
    kept
}

/// The polygon's area by the shoelace formula: positive when its points run clockwise on
/// the page (y down).
fn signed_area(points: &[Point]) -> f64 {
    edges(points)
        .map(|([ax, ay], [bx, by])| ax * by - bx * ay)
        .sum::<f64>()
        / 2.0
}

/// True when two edges that don't follow each other cross or touch. Checks every pair:
/// a few milliseconds at [`MAX_EDIT_POINTS`].
fn crosses_itself(points: &[Point]) -> bool {
    let count = points.len();
    let all: Vec<Edge> = edges(points).collect();
    all.iter().enumerate().any(|(first_index, &first)| {
        all.iter()
            .enumerate()
            .skip(first_index + 2)
            // The last edge follows on from the first.
            .filter(|(second_index, _)| !(first_index == 0 && *second_index == count - 1))
            .any(|(_, &second)| segments_meet((first, second)))
    })
}

fn edges(points: &[Point]) -> impl Iterator<Item = Edge> + '_ {
    points
        .iter()
        .copied()
        .zip(points.iter().copied().cycle().skip(1))
}

/// True when two segments cross, or one ends on the other.
fn segments_meet((first, second): (Edge, Edge)) -> bool {
    let (a, b) = (turn((first, second.0)), turn((first, second.1)));
    let (c, d) = (turn((second, first.0)), turn((second, first.1)));
    if a * b < 0.0 && c * d < 0.0 {
        return true;
    }
    (a == 0.0 && within((second.0, first)))
        || (b == 0.0 && within((second.1, first)))
        || (c == 0.0 && within((first.0, second)))
        || (d == 0.0 && within((first.1, second)))
}

/// Which side of the edge's line the point lies on: positive one way, negative the other,
/// zero on the line.
fn turn(((start, end), point): (Edge, Point)) -> f64 {
    (end[0] - start[0]) * (point[1] - start[1]) - (end[1] - start[1]) * (point[0] - start[0])
}

/// True when a point on the edge's line lies between the edge's ends.
fn within((point, (start, end)): (Point, Edge)) -> bool {
    (start[0].min(end[0])..=start[0].max(end[0])).contains(&point[0])
        && (start[1].min(end[1])..=start[1].max(end[1])).contains(&point[1])
}

#[cfg(test)]
mod tests {
    use super::*;

    const SQUARE: [Point; 4] = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];

    #[test]
    fn a_simple_outline_does_not_cross_itself() {
        assert!(!crosses_itself(&SQUARE));
        // A dent is fine.
        let dented = [
            [0.0, 0.0],
            [10.0, 0.0],
            [5.0, 4.0],
            [10.0, 10.0],
            [0.0, 10.0],
        ];
        assert!(!crosses_itself(&dented));
        // A triangle has no edges that don't follow each other.
        assert!(!crosses_itself(&[[0.0, 0.0], [10.0, 0.0], [0.0, 10.0]]));
    }

    #[test]
    fn crossing_and_touching_outlines_are_found() {
        let bow_tie = [[0.0, 0.0], [10.0, 10.0], [10.0, 0.0], [0.0, 10.0]];
        assert!(crosses_itself(&bow_tie));
        // A point dragged onto another edge.
        let touching = [
            [0.0, 0.0],
            [10.0, 0.0],
            [5.0, 10.0],
            [10.0, 10.0],
            [0.0, 10.0],
        ];
        assert!(crosses_itself(&touching));
        // The outline coming back through an earlier corner.
        let figure_eight = [
            [0.0, 0.0],
            [5.0, 5.0],
            [10.0, 0.0],
            [10.0, 10.0],
            [5.0, 5.0],
            [0.0, 10.0],
        ];
        assert!(crosses_itself(&figure_eight));
    }

    #[test]
    fn repeats_are_dropped() {
        let points = [
            [0.0, 0.0],
            [10.0, 0.0],
            [10.0, 0.0],
            [10.0, 10.0],
            [0.0, 10.0],
            [0.0, 0.0],
        ];
        assert_eq!(without_repeats(&points), SQUARE.to_vec());
    }

    #[test]
    fn the_area_has_a_sign() {
        assert_eq!(signed_area(&SQUARE), 100.0);
        let mut reversed = SQUARE;
        reversed.reverse();
        assert_eq!(signed_area(&reversed), -100.0);
    }
}
