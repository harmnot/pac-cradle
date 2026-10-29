//! The browser preview's copy of a packaging (CLAUDE.md §7.4, M1-A10).
//!
//! Production files are made only on the server, from the exact geometry with its arcs,
//! in cut and crease layers. The preview gets every drawn line as plain straight
//! segments instead: within [`PREVIEW_TOLERANCE_MM`] of the true line and with points
//! to [`PREVIEW_GRID_MM`]. That draws smooth at any zoom the editor offers, and copies of
//! one shape stay alike, but it isn't a file to cut from.
//!
//! What stays exact: sizes, placements and mounts. Users type or see those anyway, and
//! placements go back into the design while editing.
//!
//! (A 0.5 mm grid was tried first: the dense curves of a hole snapped into stair-steps
//! and copies of one hole came out different. Your review, 2026-10-29.)

use crate::geometry::{Line, Path, Point, Polygon, Segment};
use crate::packaging::Packaging;
use crate::piece::{Hinge, Panel, Piece};

/// The preview's grid: every drawn point lands on a multiple of this.
pub const PREVIEW_GRID_MM: f64 = 0.01;

/// How far a preview line may stray from the true one, before rounding to the grid.
pub const PREVIEW_TOLERANCE_MM: f64 = 0.05;

/// Grid steps per mm: rounding multiplies by this, so the result is the nearest float
/// to a whole number of hundredths (12.34, not 12.340000000000002).
const GRID_STEPS_PER_MM: f64 = 100.0;

impl Packaging {
    /// This packaging as the browser preview gets it: every piece's cut lines, creases,
    /// perforations, panels and hinge axes as straight segments, within
    /// [`PREVIEW_TOLERANCE_MM`] of the true lines, with points on the [`PREVIEW_GRID_MM`]
    /// grid.
    pub fn rounded_for_preview(&self) -> Packaging {
        let mut packaging = self.clone();
        let box_pieces = packaging
            .outer_box
            .iter_mut()
            .flat_map(|outer_box| outer_box.pieces.iter_mut());
        for piece in packaging.insert.pieces.iter_mut().chain(box_pieces) {
            round_piece(piece);
        }
        packaging
    }
}

fn round_piece(piece: &mut Piece) {
    piece.outline = round_path(&piece.outline);
    for paths in [
        &mut piece.holes,
        &mut piece.creases,
        &mut piece.perforations,
    ] {
        for path in paths.iter_mut() {
            *path = round_path(path);
        }
    }
    for panel in &mut piece.panels {
        *panel = Panel {
            outline: round_ring(panel.outline.points()),
            holes: panel
                .holes
                .iter()
                .map(|hole| round_ring(hole.points()))
                .collect(),
        };
    }
    for hinge in &mut piece.hinges {
        *hinge = Hinge {
            axis: Line {
                from: round_point(hinge.axis.from),
                to: round_point(hinge.axis.to),
            },
            ..*hinge
        };
    }
}

/// The path as straight segments between rounded points. A closed path is also
/// thinned out: the polygon library leaves curves as many short chords.
fn round_path(path: &Path) -> Path {
    let flat = path.flatten(PREVIEW_TOLERANCE_MM);
    let points = match path.closed {
        true => round_ring(&flat).0,
        false => rounded_points(&flat),
    };
    let mut points = points.into_iter();
    let start = points.next().unwrap_or(round_point(path.start));
    Path {
        start,
        segments: points.map(|to| Segment::Line { to }).collect(),
        closed: path.closed,
    }
}

/// A closed ring with the corners it can lose within [`PREVIEW_TOLERANCE_MM`] dropped,
/// then rounded, without repeats. A ring too thin to simplify keeps all its corners.
fn round_ring(points: &[Point]) -> Polygon {
    let ring = Polygon(points.to_vec());
    let simple = ring.simplify(PREVIEW_TOLERANCE_MM);
    let kept = match simple.points().len() >= 3 {
        true => simple,
        false => ring,
    };
    let mut rounded = rounded_points(kept.points());
    // A closed ring joins back to its first point by itself.
    if rounded.len() > 1 && rounded.first() == rounded.last() {
        rounded.pop();
    }
    Polygon(rounded)
}

/// Each point rounded to the grid, skipping one that lands where the last one did.
fn rounded_points(points: &[Point]) -> Vec<Point> {
    let mut rounded: Vec<Point> = Vec::with_capacity(points.len());
    for point in points.iter().copied().map(round_point) {
        if rounded.last() != Some(&point) {
            rounded.push(point);
        }
    }
    rounded
}

fn round_point(point: Point) -> Point {
    Point::new(round_mm(point.x), round_mm(point.y))
}

/// The nearest grid value. Adding 0 turns -0 into 0, so JSON never shows "-0.0".
fn round_mm(value: f64) -> f64 {
    (value * GRID_STEPS_PER_MM).round() / GRID_STEPS_PER_MM + 0.0
}
