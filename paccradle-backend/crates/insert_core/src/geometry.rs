//! Plain geometry types for the design JSON. All lengths are millimetres.
//!
//! The frame matches the vision output (CLAUDE.md §5.5) and SVG: x points right,
//! y points down. These types are the stable JSON contract stored in
//! `designs.spec`; the geometry helpers (M1-A2) convert them to `kurbo` and the
//! polygon library inside the crate, so a library upgrade never changes stored designs.

use serde::{Deserialize, Serialize};

/// A point in mm. Serialized as `[x, y]`, like `contour_mm` in the vision output.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(from = "[f64; 2]", into = "[f64; 2]")]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    /// Creates a point from x and y in mm.
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// True when both coordinates are finite numbers.
    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}

impl From<[f64; 2]> for Point {
    fn from([x, y]: [f64; 2]) -> Self {
        Self { x, y }
    }
}

impl From<Point> for [f64; 2] {
    fn from(point: Point) -> Self {
        [point.x, point.y]
    }
}

/// A closed polygon. The last point joins the first, so the first point is not repeated.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Polygon(pub Vec<Point>);

impl Polygon {
    /// The polygon's corner points.
    pub fn points(&self) -> &[Point] {
        &self.0
    }

    /// Signed area in mm² (shoelace formula). Positive when the points run clockwise on screen.
    pub fn signed_area_mm2(&self) -> f64 {
        let points = &self.0;
        let twice_area: f64 = points
            .iter()
            .zip(points.iter().cycle().skip(1))
            .map(|(a, b)| a.x * b.y - b.x * a.y)
            .sum();
        twice_area / 2.0
    }
}

/// A straight line from `from` to `to`, used for hinge axes.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Line {
    pub from: Point,
    pub to: Point,
}

/// A cut, crease or perforation line made of straight lines and circular arcs.
///
/// Arcs stay arcs (not polylines) so the DXF and PDF exports keep exact curves for die makers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Path {
    /// Where the path starts.
    pub start: Point,
    /// Segments in drawing order; each one starts where the previous one ended.
    pub segments: Vec<Segment>,
    /// True when the path joins back to `start` (outlines and holes).
    pub closed: bool,
}

/// One piece of a [`Path`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Segment {
    /// Straight line from the current point to `to`.
    Line { to: Point },
    /// Circular arc from the current point around `center`, turning by `sweep_deg`.
    ///
    /// The radius is the distance from the current point to `center`, so it can't
    /// disagree with the end point. Positive angles turn from +x towards +y, which is
    /// clockwise on screen because y points down. 360 draws a full circle.
    Arc { center: Point, sweep_deg: f64 },
}
