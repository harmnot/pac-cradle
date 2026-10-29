//! The hole one product sits in: its outline grown by the clearance, plus optional
//! finger notches. Built once per product, then placed for every copy.

use crate::design::{FingerNotches, NotchSide, Placement, Product};
use crate::error::GeometryError;
use crate::geometry::{Area, Bounds, CURVE_TOLERANCE_MM, Path, Point, Polygon, Transform};

/// Radius of a finger notch: 19 mm across, about a fingertip (¾ inch, a common size in
/// packaging). The notch reaches this far past the hole's edge.
pub const FINGER_NOTCH_RADIUS_MM: f64 = 9.5;

/// How a product's hole is cut.
#[derive(Debug, Clone, Copy)]
pub(super) struct HoleStyle {
    /// Gap between the product and the board, per side.
    pub clearance_mm: f64,
    /// Half-round notches on a long side, to get a finger beside the product.
    pub finger_notches: Option<FingerNotches>,
}

/// A long side of a straightened outline (long side along x).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Edge {
    /// Larger y: lower on screen.
    Bottom,
    Top,
}

impl Edge {
    /// The sides a notch setting puts notches on.
    fn of(side: NotchSide) -> &'static [Self] {
        match side {
            NotchSide::Bottom => &[Self::Bottom],
            NotchSide::Top => &[Self::Top],
            NotchSide::Both => &[Self::Bottom, Self::Top],
        }
    }

    /// Which way is out of the outline across this side, along y.
    fn outward(self) -> f64 {
        match self {
            Self::Bottom => 1.0,
            Self::Top => -1.0,
        }
    }
}

/// Which way a straightened product lies on the card.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Lie {
    /// Long side along x.
    Lengthwise,
    /// Long side along y: a quarter turn from lengthwise.
    Crosswise,
}

impl Lie {
    fn turn_deg(self) -> f64 {
        match self {
            Self::Lengthwise => 0.0,
            Self::Crosswise => 90.0,
        }
    }
}

/// One product's hole, ready to place.
pub(super) struct ProductHole {
    /// Turn that sets the product's outline straight (see
    /// [`Polygon::straightening_turn_deg`]).
    straightening_deg: f64,
    /// The hole with the product lying lengthwise and the centre of its outline's
    /// bounding box (taken before straightening) at the origin.
    straight: Area,
    /// The product's own outline, placed the same way (no clearance, no notches).
    straight_outline: Polygon,
}

impl ProductHole {
    /// Builds the hole for `product`.
    pub(super) fn new(product: &Product, style: HoleStyle) -> Result<Self, GeometryError> {
        let outline = &product.contour_mm;
        let straightening_deg = outline.straightening_turn_deg();
        let pivot = outline.bounds().map_or(Point::ORIGIN, Bounds::center);
        let to_straight =
            Transform::shift(Point::ORIGIN - pivot).then(Transform::turn(straightening_deg));
        let straight_outline = outline.transformed(to_straight);
        let mut straight = Area::from_polygon(&straight_outline)?.offset(style.clearance_mm)?;
        if style.finger_notches.is_some() {
            straight = straight.union(&finger_notches((&straight_outline, style))?)?;
        }
        Ok(Self {
            straightening_deg,
            straight,
            straight_outline,
        })
    }

    /// The hole where `placement` puts the product.
    pub(super) fn placed(&self, placement: &Placement) -> Area {
        self.straight.transformed(self.placing(placement))
    }

    /// The product's own outline (no clearance, no notches) where `placement` puts it.
    pub(super) fn placed_outline(&self, placement: &Placement) -> Polygon {
        self.straight_outline.transformed(self.placing(placement))
    }

    /// From the straightened hole to where `placement` puts it.
    fn placing(&self, placement: &Placement) -> Transform {
        Transform {
            turn_deg: placement.rotation_deg - self.straightening_deg,
            shift: placement.center_mm,
        }
    }

    /// The `rotation_deg` that makes a placement lay the product straight, lying `lie`.
    pub(super) fn rotation_deg(&self, lie: Lie) -> f64 {
        half_circle_range(self.straightening_deg + lie.turn_deg())
    }

    /// The box around the hole of a product lying `lie`, for a placement centred on the
    /// origin.
    pub(super) fn footprint(&self, lie: Lie) -> Option<Bounds> {
        self.straight
            .transformed(Transform::turn(lie.turn_deg()))
            .bounds()
    }

    /// The box around the product itself (no clearance) lying `lie`, for a placement
    /// centred on the origin. Exact, unlike the hole's rounded corners.
    pub(super) fn product_footprint(&self, lie: Lie) -> Option<Bounds> {
        self.straight_outline
            .transformed(Transform::turn(lie.turn_deg()))
            .bounds()
    }
}

/// The notches for a straightened outline: each one centred on the hole's edge, on the
/// chosen side(s), `along` the way from the outline's left end to its right end.
fn finger_notches((outline, style): (&Polygon, HoleStyle)) -> Result<Area, GeometryError> {
    let (Some(notches), Some(bounds)) = (style.finger_notches, outline.bounds()) else {
        return Ok(Area::default());
    };
    let x = bounds.min.x + notches.along * bounds.length_mm();
    let circles: Vec<Polygon> = Edge::of(notches.side)
        .iter()
        .map(|&edge| {
            let fallback = match edge {
                Edge::Bottom => bounds.max.y,
                Edge::Top => bounds.min.y,
            };
            let edge_y = edge_crossing((outline, x), edge).unwrap_or(fallback);
            let center = Point::new(x, edge_y + edge.outward() * style.clearance_mm);
            Polygon(Path::circle(center, FINGER_NOTCH_RADIUS_MM).flatten(CURVE_TOLERANCE_MM))
        })
        .collect();
    Area::from_polygons(&circles)
}

/// Where the vertical line at `x` meets the outline's `edge`: the largest y for the
/// bottom, the smallest for the top.
fn edge_crossing((outline, x): (&Polygon, f64), edge: Edge) -> Option<f64> {
    let points = outline.points();
    let crossings = points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .filter(|(from, to)| from.x.min(to.x) <= x && x <= from.x.max(to.x))
        .map(|(from, to)| match (from.x == to.x, edge) {
            (true, Edge::Bottom) => from.y.max(to.y),
            (true, Edge::Top) => from.y.min(to.y),
            (false, _) => from.y + (x - from.x) * (to.y - from.y) / (to.x - from.x),
        });
    match edge {
        Edge::Bottom => crossings.max_by(f64::total_cmp),
        Edge::Top => crossings.min_by(f64::total_cmp),
    }
}

/// The same turn as `turn_deg`, more than −180 and at most 180.
fn half_circle_range(turn_deg: f64) -> f64 {
    let turned = turn_deg.rem_euclid(360.0);
    match turned > 180.0 {
        true => turned - 360.0,
        false => turned,
    }
}
