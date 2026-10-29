//! Turning and moving outlines: how products are placed on a card or deck.

use super::{Point, Polygon};

/// A turn about the origin, then a shift.
///
/// It never mirrors, so a clean [`Area`](super::Area) stays clean and rings keep the
/// direction they run in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    /// Positive turns from +x towards +y: clockwise on screen, like
    /// [`Placement::rotation_deg`](crate::Placement::rotation_deg).
    pub turn_deg: f64,
    /// Added after the turn.
    pub shift: Point,
}

impl Transform {
    /// Leaves points where they are.
    pub const IDENTITY: Self = Self::turn(0.0);

    /// A turn about the origin and no shift.
    pub const fn turn(turn_deg: f64) -> Self {
        Self {
            turn_deg,
            shift: Point::ORIGIN,
        }
    }

    /// A shift and no turn.
    pub const fn shift(shift: Point) -> Self {
        Self {
            turn_deg: 0.0,
            shift,
        }
    }

    /// Where `point` ends up.
    pub fn apply(self, point: Point) -> Point {
        let (sin, cos) = self.turn_deg.to_radians().sin_cos();
        Point::new(
            point.x * cos - point.y * sin + self.shift.x,
            point.x * sin + point.y * cos + self.shift.y,
        )
    }

    /// This transform followed by `next`, as one transform.
    pub fn then(self, next: Self) -> Self {
        Self {
            turn_deg: self.turn_deg + next.turn_deg,
            shift: next.apply(self.shift),
        }
    }
}

impl Polygon {
    /// The polygon with every point moved by `transform`.
    pub fn transformed(&self, transform: Transform) -> Polygon {
        Polygon(
            self.points()
                .iter()
                .map(|&point| transform.apply(point))
                .collect(),
        )
    }
}
