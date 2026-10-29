//! Closed rings of straight edges, ready for exact distance checks: the hole spacing
//! rules (M1-A7) and the fit check (A8). Each edge keeps its box, so pairs of edges
//! already too far apart are skipped without measuring.

use super::{Bounds, Line, Point, Polygon};

/// Straight lines with their boxes.
pub(crate) type Edges = [(Line, Bounds)];

/// A closed ring of straight edges.
pub(crate) struct Ring {
    pub polygon: Polygon,
    pub edges: Vec<(Line, Bounds)>,
    pub bounds: Bounds,
}

impl Ring {
    /// The ring round `polygon`, or `None` when it has no points.
    pub(crate) fn new(polygon: Polygon) -> Option<Self> {
        let bounds = polygon.bounds()?;
        let edges = polygon.edges().map(|edge| (edge, edge.bounds())).collect();
        Some(Self {
            polygon,
            edges,
            bounds,
        })
    }

    /// A point on the ring, for inside-or-outside tests.
    pub(crate) fn first(&self) -> Point {
        self.polygon
            .points()
            .first()
            .copied()
            .unwrap_or(Point::ORIGIN)
    }

    /// The gap between the two rings' areas, up to `limit`: 0 when they cross, touch or
    /// one lies inside the other.
    pub(crate) fn gap_to(&self, (other, limit): (&Ring, f64)) -> f64 {
        if self.bounds.gap_to(other.bounds) >= limit {
            return limit;
        }
        if self.polygon.contains(other.first()) || other.polygon.contains(self.first()) {
            return 0.0;
        }
        edges_gap((&self.edges, &other.edges, limit))
    }

    /// The shortest distance between the two rings' lines, up to `limit`: 0 when they
    /// cross or touch. Which ring lies inside which doesn't matter here.
    pub(crate) fn line_gap(&self, (other, limit): (&Ring, f64)) -> f64 {
        if self.bounds.gap_to(other.bounds) >= limit {
            return limit;
        }
        edges_gap((&self.edges, &other.edges, limit))
    }
}

/// The shortest distance between two sets of lines, up to `limit`: line pairs whose
/// boxes are already that far apart are skipped.
pub(crate) fn edges_gap((a, b, limit): (&Edges, &Edges, f64)) -> f64 {
    let mut best = limit;
    for &(line, bounds) in a {
        for &(other, other_bounds) in b {
            if bounds.gap_to(other_bounds) >= best {
                continue;
            }
            best = best.min(line.distance_to_line(other));
            if best <= 0.0 {
                return 0.0;
            }
        }
    }
    best
}
