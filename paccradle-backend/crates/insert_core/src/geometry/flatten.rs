//! Converts between [`Path`]s (lines and arcs) and [`Polygon`]s (corners only).

use std::f64::consts::FRAC_PI_2;

use super::{Path, Point, Polygon, Segment};

/// Finest tolerance accepted: the 0.1 µm grid the polygon library snaps to.
const MIN_TOLERANCE_MM: f64 = 0.0001;
/// Most chords one arc becomes. Caps the work for absurd radii; at
/// [`CURVE_TOLERANCE_MM`](super::CURVE_TOLERANCE_MM), a full circle 2 m across needs about 2,200.
const MAX_ARC_CHORDS: f64 = 4096.0;
/// Points closer than this are the same point (floating-point noise at an arc's end).
const SAME_POINT_MM: f64 = 1e-9;

impl Path {
    /// A full circle drawn as one 360° arc, starting at its rightmost point.
    pub fn circle(center: Point, radius_mm: f64) -> Path {
        Path {
            start: Point::new(center.x + radius_mm, center.y),
            segments: vec![Segment::Arc {
                center,
                sweep_deg: 360.0,
            }],
            closed: true,
        }
    }

    /// The path's corner points, with each arc replaced by chords that stray at most
    /// `tolerance_mm` from it (at least 0.0001 mm).
    ///
    /// A closed path doesn't repeat its start point at the end, so
    /// `Polygon(path.flatten(tolerance))` is its outline.
    pub fn flatten(&self, tolerance_mm: f64) -> Vec<Point> {
        let tolerance_mm = tolerance_mm.max(MIN_TOLERANCE_MM);
        let mut points = vec![self.start];
        let mut current = self.start;
        for segment in &self.segments {
            match *segment {
                Segment::Line { to } => points.push(to),
                Segment::Arc { center, sweep_deg } => {
                    let arc = Arc {
                        start: current,
                        center,
                        sweep_rad: sweep_deg.to_radians(),
                    };
                    points.extend(arc.chord_ends(tolerance_mm));
                }
            }
            current = points.last().copied().unwrap_or(current);
        }
        let back_at_start = current.distance_to(self.start) <= SAME_POINT_MM;
        if self.closed && back_at_start && points.len() > 1 {
            points.pop();
        }
        points
    }
}

impl Polygon {
    /// The polygon as a closed path of straight lines, ready for cut and crease output.
    /// `None` when the polygon has no points.
    pub fn to_path(&self) -> Option<Path> {
        let (start, rest) = self.points().split_first()?;
        Some(Path {
            start: *start,
            segments: rest.iter().map(|&to| Segment::Line { to }).collect(),
            closed: true,
        })
    }
}

/// Largest angle (radians) an arc of `radius_mm` may turn per chord so that each chord
/// stays within `tolerance_mm` of the arc. At most a quarter turn.
pub(super) fn chord_angle(radius_mm: f64, tolerance_mm: f64) -> f64 {
    if tolerance_mm >= radius_mm {
        return FRAC_PI_2;
    }
    // A chord turning by `angle` sits radius × (1 − cos(angle / 2)) inside the arc at its middle.
    (2.0 * (1.0 - tolerance_mm / radius_mm).acos()).min(FRAC_PI_2)
}

/// A circular arc from `start` around `center`, turning by `sweep_rad`
/// (positive turns from +x towards +y: clockwise on screen).
struct Arc {
    start: Point,
    center: Point,
    sweep_rad: f64,
}

impl Arc {
    /// The end of each chord in order, so the last one is the arc's end point.
    fn chord_ends(&self, tolerance_mm: f64) -> impl Iterator<Item = Point> {
        let (dx, dy) = (self.start.x - self.center.x, self.start.y - self.center.y);
        let radius = dx.hypot(dy);
        let start_angle = dy.atan2(dx);
        let chords = (self.sweep_rad.abs() / chord_angle(radius, tolerance_mm))
            .ceil()
            .clamp(1.0, MAX_ARC_CHORDS) as u32;
        // A sweep that isn't a number casts to 0 chords; keep one so the path still moves on.
        let chords = chords.max(1);
        let step = self.sweep_rad / f64::from(chords);
        let center = self.center;
        (1..=chords).map(move |chord| {
            let angle = start_angle + step * f64::from(chord);
            Point::new(
                center.x + radius * angle.cos(),
                center.y + radius * angle.sin(),
            )
        })
    }
}
