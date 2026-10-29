//! Axis-aligned bounding boxes.

use serde::{Deserialize, Serialize};

use super::{Line, Point, Polygon};

/// The smallest axis-aligned rectangle around some points.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Bounds {
    /// Top-left corner (smallest x and y).
    pub min: Point,
    /// Bottom-right corner (largest x and y).
    pub max: Point,
}

impl Bounds {
    /// The box from (0, 0) to (`length_mm`, `width_mm`): a card or panel of that size.
    pub fn from_size((length_mm, width_mm): (f64, f64)) -> Self {
        Self {
            min: Point::ORIGIN,
            max: Point::new(length_mm, width_mm),
        }
    }

    /// The box around `points`, or `None` when there are none.
    pub fn of(points: impl IntoIterator<Item = Point>) -> Option<Self> {
        points.into_iter().fold(None, |bounds, point| {
            let around_point = Self {
                min: point,
                max: point,
            };
            Some(bounds.map_or(around_point, |bounds: Self| bounds.union(around_point)))
        })
    }

    /// Extent along x.
    pub fn length_mm(self) -> f64 {
        self.max.x - self.min.x
    }

    /// Extent along y.
    pub fn width_mm(self) -> f64 {
        self.max.y - self.min.y
    }

    /// The middle of the box.
    pub fn center(self) -> Point {
        Point::new(
            (self.min.x + self.max.x) / 2.0,
            (self.min.y + self.max.y) / 2.0,
        )
    }

    /// The box around both boxes.
    pub fn union(self, other: Self) -> Self {
        Self {
            min: Point::new(self.min.x.min(other.min.x), self.min.y.min(other.min.y)),
            max: Point::new(self.max.x.max(other.max.x), self.max.y.max(other.max.y)),
        }
    }

    /// The box grown by `by_mm` on every side, or shrunk when negative.
    pub fn expanded(self, by_mm: f64) -> Self {
        Self {
            min: Point::new(self.min.x - by_mm, self.min.y - by_mm),
            max: Point::new(self.max.x + by_mm, self.max.y + by_mm),
        }
    }

    /// How far apart the two boxes are: 0 when they touch or overlap.
    pub fn gap_to(self, other: Self) -> f64 {
        let dx = (other.min.x - self.max.x)
            .max(self.min.x - other.max.x)
            .max(0.0);
        let dy = (other.min.y - self.max.y)
            .max(self.min.y - other.max.y)
            .max(0.0);
        dx.hypot(dy)
    }

    /// True when `other` lies entirely inside this box (edges included).
    pub fn encloses(self, other: Self) -> bool {
        other.min.x >= self.min.x
            && other.min.y >= self.min.y
            && other.max.x <= self.max.x
            && other.max.y <= self.max.y
    }

    /// The box as a rectangle running clockwise on screen, like outer rings.
    pub fn to_polygon(self) -> Polygon {
        Polygon(vec![
            self.min,
            Point::new(self.max.x, self.min.y),
            self.max,
            Point::new(self.min.x, self.max.y),
        ])
    }
}

impl Line {
    /// The box around the line.
    pub fn bounds(self) -> Bounds {
        Bounds {
            min: Point::new(self.from.x.min(self.to.x), self.from.y.min(self.to.y)),
            max: Point::new(self.from.x.max(self.to.x), self.from.y.max(self.to.y)),
        }
    }
}

impl Polygon {
    /// The box around the polygon's points, or `None` when it has none.
    pub fn bounds(&self) -> Option<Bounds> {
        Bounds::of(self.points().iter().copied())
    }
}
