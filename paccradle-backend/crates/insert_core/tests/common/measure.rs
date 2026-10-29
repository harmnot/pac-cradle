//! Shapes and measuring helpers for the geometry tests.

use insert_core::{Area, Line, Path, Point, Polygon, Segment};

/// Points checked along each edge. Chords are checked in the middle, not just at corners.
const SAMPLES_PER_EDGE: u32 = 16;

/// Axis-aligned rectangle with its top-left corner at `corner`.
pub fn rectangle_at(corner: Point, size_mm: (f64, f64)) -> Polygon {
    let (length, width) = size_mm;
    Polygon(vec![
        corner,
        Point::new(corner.x + length, corner.y),
        Point::new(corner.x + length, corner.y + width),
        Point::new(corner.x, corner.y + width),
    ])
}

/// A full circle around the origin as one 360° arc, the way generators draw round holes.
pub fn circle_path(radius_mm: f64) -> Path {
    Path {
        start: Point::new(radius_mm, 0.0),
        segments: vec![Segment::Arc {
            center: Point::new(0.0, 0.0),
            sweep_deg: 360.0,
        }],
        closed: true,
    }
}

/// An L: 60 wide, 50 tall, arms 20 and 25 thick. Its inside corner is at (25, 20).
pub fn l_shape() -> Polygon {
    Polygon(
        [
            [0.0, 0.0],
            [60.0, 0.0],
            [60.0, 20.0],
            [25.0, 20.0],
            [25.0, 50.0],
            [0.0, 50.0],
        ]
        .map(Point::from)
        .to_vec(),
    )
}

/// Two 30 mm squares joined by a 10 mm long, 4 mm wide neck.
pub fn dumbbell() -> Polygon {
    Polygon(
        [
            [0.0, 0.0],
            [30.0, 0.0],
            [30.0, 13.0],
            [40.0, 13.0],
            [40.0, 0.0],
            [70.0, 0.0],
            [70.0, 30.0],
            [40.0, 30.0],
            [40.0, 17.0],
            [30.0, 17.0],
            [30.0, 30.0],
            [0.0, 30.0],
        ]
        .map(Point::from)
        .to_vec(),
    )
}

/// The edges of a ring, including the one from the last point back to the first.
pub fn edges(ring: &Polygon) -> impl Iterator<Item = Line> + '_ {
    let points = ring.points();
    points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .map(|(&from, &to)| Line { from, to })
}

/// Shortest distance from `point` to the outline of `ring`.
pub fn distance_to_outline(point: Point, ring: &Polygon) -> f64 {
    edges(ring)
        .map(|edge| edge.distance_to(point))
        .fold(f64::INFINITY, f64::min)
}

/// True when `point` is inside `ring` (even-odd rule).
pub fn is_inside(point: Point, ring: &Polygon) -> bool {
    edges(ring)
        .filter(|edge| (edge.from.y > point.y) != (edge.to.y > point.y))
        .filter(|edge| {
            let crossing_x = edge.from.x
                + (point.y - edge.from.y) * (edge.to.x - edge.from.x) / (edge.to.y - edge.from.y);
            point.x < crossing_x
        })
        .count()
        % 2
        == 1
}

/// Points spread along every boundary ring of `area`: outer rings and holes.
pub fn boundary_points(area: &Area) -> Vec<Point> {
    let rings = area
        .regions()
        .iter()
        .flat_map(|region| std::iter::once(&region.outer).chain(&region.holes));
    rings
        .flat_map(edges)
        .flat_map(|edge| {
            (0..SAMPLES_PER_EDGE).map(move |step| {
                let along = f64::from(step) / f64::from(SAMPLES_PER_EDGE);
                Point::new(
                    edge.from.x + along * (edge.to.x - edge.from.x),
                    edge.from.y + along * (edge.to.y - edge.from.y),
                )
            })
        })
        .collect()
}

/// How an offset of a polygon compares with the exact offset.
pub struct OffsetCheck {
    pub result: Area,
    /// Worst gap between the wanted distance and a boundary point's real distance to the original.
    pub worst_error_mm: f64,
    /// Boundary points on the wrong side: inside the original after growing, outside after shrinking.
    pub wrong_side: usize,
}

/// Offsets `original` by `distance_mm` and measures every boundary point of the result.
pub fn check_offset(original: &Polygon, distance_mm: f64) -> OffsetCheck {
    let result = match Area::from_polygon(original).and_then(|area| area.offset(distance_mm)) {
        Ok(result) => result,
        Err(error) => panic!("offset by {distance_mm} mm: {error}"),
    };
    let points = boundary_points(&result);
    let worst_error_mm = points
        .iter()
        .map(|&point| (distance_to_outline(point, original) - distance_mm.abs()).abs())
        .fold(0.0, f64::max);
    let wrong_side = points
        .iter()
        .filter(|&&point| is_inside(point, original) != (distance_mm < 0.0))
        .count();
    OffsetCheck {
        result,
        worst_error_mm,
        wrong_side,
    }
}

/// True when some corner of the area's rings lies within `within_mm` of `point`.
pub fn has_corner_near(area: &Area, (point, within_mm): (Point, f64)) -> bool {
    area.regions()
        .iter()
        .flat_map(|region| std::iter::once(&region.outer).chain(&region.holes))
        .flat_map(Polygon::points)
        .any(|corner| corner.distance_to(point) <= within_mm)
}
