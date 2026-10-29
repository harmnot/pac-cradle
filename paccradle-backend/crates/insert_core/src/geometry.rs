//! Plain geometry types for the design JSON, and the helpers the generators build on.
//! All lengths are millimetres.
//!
//! The frame matches the vision output (CLAUDE.md §5.5) and SVG: x points right,
//! y points down. These types are the stable JSON contract stored in `designs.spec`.
//!
//! Helpers (M1-A2): [`Path::flatten`] turns arcs into chords, [`Polygon::simplify`]
//! drops needless corners, and [`Area`] offsets and combines shapes. Only `area` uses
//! the polygon library (`i_overlay`), so a library upgrade never changes stored designs.
//! Added for the generators (M1-A3): [`Bounds`], [`Transform`] and
//! [`Polygon::straightening_turn_deg`]. Added for the checks (M1-A7):
//! [`Polygon::contains`] and [`Line::distance_to_line`].

mod area;
mod bounds;
mod flatten;
mod hull;
mod rings;
mod simplify;
mod transform;

use std::ops::{Add, Sub};

use serde::{Deserialize, Serialize};

pub use area::{Area, Region};
pub use bounds::Bounds;
pub use transform::Transform;

pub(crate) use rings::{Ring, edges_gap};

/// How far a straight chord may stray from the curve it stands in for: arcs in
/// [`Path::flatten`] and the round corners of [`Area::offset`].
///
/// 0.002 mm keeps offsets well inside the 0.01 mm accuracy target and is far finer
/// than any cutter (laser kerf is about 0.1–0.2 mm).
pub const CURVE_TOLERANCE_MM: f64 = 0.002;

/// A point in mm. Serialized as `[x, y]`, like `contour_mm` in the vision output.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(from = "[f64; 2]", into = "[f64; 2]")]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    /// The point (0, 0).
    pub const ORIGIN: Self = Self::new(0.0, 0.0);

    /// Creates a point from x and y in mm.
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// True when both coordinates are finite numbers.
    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }

    /// Straight-line distance to `other` in mm.
    pub fn distance_to(self, other: Point) -> f64 {
        (other.x - self.x).hypot(other.y - self.y)
    }
}

/// Adds coordinates: moves a point by an offset.
impl Add for Point {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self::new(self.x + other.x, self.y + other.y)
    }
}

/// Subtracts coordinates: the offset from `other` to `self`.
impl Sub for Point {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self::new(self.x - other.x, self.y - other.y)
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

    /// The polygon's edges, including the one from the last point back to the first.
    pub fn edges(&self) -> impl Iterator<Item = Line> + '_ {
        let points = self.points();
        points
            .iter()
            .zip(points.iter().cycle().skip(1))
            .map(|(&from, &to)| Line { from, to })
    }

    /// True when `point` lies inside the polygon (even-odd rule). A point exactly on an
    /// edge may count either way.
    pub fn contains(&self, point: Point) -> bool {
        self.edges()
            .filter(|edge| (edge.from.y > point.y) != (edge.to.y > point.y))
            .filter(|edge| {
                let crossing_x = edge.from.x
                    + (point.y - edge.from.y) * (edge.to.x - edge.from.x)
                        / (edge.to.y - edge.from.y);
                point.x < crossing_x
            })
            .count()
            % 2
            == 1
    }
}

/// A straight line from `from` to `to`: hinge axes, and polygon edges in the helpers.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Line {
    pub from: Point,
    pub to: Point,
}

impl Line {
    /// Shortest distance from `point` to this line segment (not the endless line through it).
    pub fn distance_to(self, point: Point) -> f64 {
        let (dx, dy) = (self.to.x - self.from.x, self.to.y - self.from.y);
        let length_squared = dx * dx + dy * dy;
        if length_squared == 0.0 {
            return self.from.distance_to(point);
        }
        // How far along the segment the closest point sits, 0 at `from` and 1 at `to`.
        let along = ((point.x - self.from.x) * dx + (point.y - self.from.y) * dy) / length_squared;
        let along = along.clamp(0.0, 1.0);
        Point::new(self.from.x + along * dx, self.from.y + along * dy).distance_to(point)
    }

    /// Shortest distance between this line segment and `other`: 0 when they cross or
    /// touch.
    pub fn distance_to_line(self, other: Line) -> f64 {
        if self.crosses(other) {
            return 0.0;
        }
        // Segments that don't cross come closest at one of their ends.
        [
            other.distance_to(self.from),
            other.distance_to(self.to),
            self.distance_to(other.from),
            self.distance_to(other.to),
        ]
        .into_iter()
        .fold(f64::INFINITY, f64::min)
    }

    /// True when each segment's ends lie strictly on opposite sides of the other.
    fn crosses(self, other: Line) -> bool {
        let side = |line: Line, point: Point| {
            (line.to.x - line.from.x) * (point.y - line.from.y)
                - (line.to.y - line.from.y) * (point.x - line.from.x)
        };
        let apart = |a: f64, b: f64| (a > 0.0 && b < 0.0) || (a < 0.0 && b > 0.0);
        apart(side(other, self.from), side(other, self.to))
            && apart(side(self, other.from), side(self, other.to))
    }
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
