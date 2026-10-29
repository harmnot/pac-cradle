//! Convex hulls, and the turn that sets a photographed outline straight.
//!
//! Vision keeps each outline at the angle it lay on the paper (CLAUDE.md §5.3), so a box
//! photographed at 30° needs turning back before it's laid out in rows.

use super::{Point, Polygon, Transform};

/// Rectangles whose areas differ by less than this share count as equally small, so near
/// round outlines pick the smallest turn instead of a noisy one.
const SAME_AREA_RATIO: f64 = 1e-3;

impl Polygon {
    /// The smallest convex polygon around the points (Andrew's monotone chain), running
    /// clockwise on screen. Points on its straight edges are left out.
    pub fn convex_hull(&self) -> Polygon {
        let mut points = self.points().to_vec();
        points.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
        points.dedup();
        if points.len() < 3 {
            return Polygon(points);
        }
        let mut hull = half_hull(points.iter());
        hull.pop();
        let mut other_half = half_hull(points.iter().rev());
        other_half.pop();
        hull.extend(other_half);
        Polygon(hull)
    }

    /// The turn in degrees (clockwise on screen, more than −90 and at most 90) that sets
    /// the outline straight: its smallest enclosing rectangle lines up with the axes, long
    /// side along x. Use it as a placement's `rotation_deg` to lay the product straight.
    pub fn straightening_turn_deg(&self) -> f64 {
        let hull = self.convex_hull();
        let points = hull.points();
        let candidates = points
            .iter()
            .zip(points.iter().cycle().skip(1))
            .filter(|(from, to)| from != to)
            .filter_map(|(&from, &to)| Candidate::along(&hull, (from, to)));
        let mut candidates: Vec<Candidate> = candidates.collect();
        let Some(smallest) = candidates
            .iter()
            .map(|candidate| candidate.area_mm2)
            .min_by(f64::total_cmp)
        else {
            return 0.0;
        };
        let near_smallest = smallest * (1.0 + SAME_AREA_RATIO);
        candidates.retain(|candidate| candidate.area_mm2 <= near_smallest);
        candidates
            .iter()
            .map(|candidate| candidate.turn_deg)
            .min_by(|a, b| a.abs().total_cmp(&b.abs()))
            .unwrap_or(0.0)
    }
}

/// One half of the hull: keeps only left turns while walking the sorted points.
fn half_hull<'a>(points: impl Iterator<Item = &'a Point>) -> Vec<Point> {
    let mut hull: Vec<Point> = Vec::new();
    for &point in points {
        while let [.., before, last] = hull.as_slice() {
            if cross((*before, *last), point) > 0.0 {
                break;
            }
            hull.pop();
        }
        hull.push(point);
    }
    hull
}

/// Positive when `point` lies to the right of the line `from → to`, as seen on screen.
fn cross((from, to): (Point, Point), point: Point) -> f64 {
    let (a, b) = (to - from, point - from);
    a.x * b.y - a.y * b.x
}

/// An enclosing rectangle lined up with one hull edge.
struct Candidate {
    /// Turn that lines the rectangle up with the axes, long side along x.
    turn_deg: f64,
    area_mm2: f64,
}

impl Candidate {
    fn along(hull: &Polygon, (from, to): (Point, Point)) -> Option<Self> {
        let edge = to - from;
        let edge_turn = -edge.y.atan2(edge.x).to_degrees();
        let bounds = hull.transformed(Transform::turn(edge_turn)).bounds()?;
        let long_side_on_y = bounds.width_mm() > bounds.length_mm();
        let turn_deg = match long_side_on_y {
            true => edge_turn + 90.0,
            false => edge_turn,
        };
        Some(Self {
            turn_deg: half_turn_range(turn_deg),
            area_mm2: bounds.length_mm() * bounds.width_mm(),
        })
    }
}

/// The same rectangle orientation as `turn_deg`, more than −90 and at most 90.
fn half_turn_range(turn_deg: f64) -> f64 {
    let turned = turn_deg.rem_euclid(180.0);
    match turned > 90.0 {
        true => turned - 180.0,
        false => turned,
    }
}
