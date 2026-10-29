//! Filled areas: offsets and booleans (union, intersection, difference) through `i_overlay`.
//!
//! Inside the library, coordinates snap to a fixed 0.1 µm grid of 32-bit integers, so
//! results don't depend on a shape's size and come out the same natively and in WASM.
//! Booleans put the grid's origin on a whole millimetre near the shapes, so every
//! operation snaps a coordinate to the same grid line: shapes built apart that share an
//! edge still share it when joined (M1-A9). Everything in one operation must fit in
//! about 100 m across.

use std::iter;

use i_overlay::core::fill_rule::FillRule;
use i_overlay::core::overlay::ShapeType;
use i_overlay::core::overlay_rule::OverlayRule;
use i_overlay::float::overlay::FloatOverlay;
use i_overlay::i_float::adapter::FloatPointAdapter;
use i_overlay::i_float::float::compatible::FloatPointCompatible;
use i_overlay::i_float::float::rect::FloatRect;
use i_overlay::mesh::float::outline::offset::OutlineOffset;
use i_overlay::mesh::float::style::{LineJoin, OutlineStyle};

use super::flatten::chord_angle;
use super::{Bounds, CURVE_TOLERANCE_MM, Point, Polygon, Transform};
use crate::error::GeometryError;

/// Library grid steps per mm (0.1 µm each).
const GRID_PER_MM: f64 = 10_000.0;

/// The library's form of an area: shapes, each an outer ring followed by its holes.
type Shapes = Vec<Vec<Vec<Point>>>;

impl FloatPointCompatible for Point {
    type Scalar = f64;

    fn from_xy(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    fn x(&self) -> f64 {
        self.x
    }

    fn y(&self) -> f64 {
        self.y
    }
}

/// One connected filled region: an outer ring and the holes cut out of it.
#[derive(Debug, Clone, PartialEq)]
pub struct Region {
    /// Runs clockwise on screen (positive [`Polygon::signed_area_mm2`]).
    pub outer: Polygon,
    /// Each runs anticlockwise on screen (negative signed area) and sits inside `outer`.
    pub holes: Vec<Polygon>,
}

impl Region {
    /// Filled area in mm²: the outer ring minus the holes.
    pub fn area_mm2(&self) -> f64 {
        let holes: f64 = self
            .holes
            .iter()
            .map(|hole| hole.signed_area_mm2().abs())
            .sum();
        self.outer.signed_area_mm2().abs() - holes
    }
}

/// A filled area made of separate [`Region`]s: what offsets and booleans take and return.
///
/// An area is always clean: regions don't cross themselves or each other, and rings run
/// the way [`Region`] says. It can only be built through [`Area::from_polygon`],
/// [`Area::from_polygons`] or an operation, which keeps it that way. `Area::default()`
/// is empty.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Area {
    regions: Vec<Region>,
}

impl Area {
    /// The area inside one outline, whichever way it runs. An outline that crosses
    /// itself, like a figure eight, keeps every loop.
    pub fn from_polygon(polygon: &Polygon) -> Result<Self, GeometryError> {
        Self::from_polygons(std::slice::from_ref(polygon))
    }

    /// The area covered by any of the outlines (their union), whichever way each runs.
    /// Build a region with a hole by subtracting: `outer.difference(&hole)`.
    pub fn from_polygons(polygons: &[Polygon]) -> Result<Self, GeometryError> {
        let rings = polygons.iter().map(clockwise_points).collect();
        overlay((&vec![rings], &Shapes::new()), OverlayRule::Subject)
    }

    /// Everything covered by any of the areas, in one pass (faster than joining them one
    /// by one).
    pub fn union_all<'a>(areas: impl IntoIterator<Item = &'a Area>) -> Result<Self, GeometryError> {
        // Each area is clean, so where any of them covers, the windings add up to at least 1.
        let shapes: Shapes = areas.into_iter().flat_map(Self::to_shapes).collect();
        overlay((&shapes, &Shapes::new()), OverlayRule::Subject)
    }

    /// The separate regions, in no particular order.
    pub fn regions(&self) -> &[Region] {
        &self.regions
    }

    /// Takes the regions out of the area.
    pub fn into_regions(self) -> Vec<Region> {
        self.regions
    }

    /// True when the area covers nothing.
    pub fn is_empty(&self) -> bool {
        self.regions.is_empty()
    }

    /// Total filled area in mm².
    pub fn area_mm2(&self) -> f64 {
        self.regions.iter().map(Region::area_mm2).sum()
    }

    /// The box around the area, or `None` when it's empty.
    pub fn bounds(&self) -> Option<Bounds> {
        Bounds::of(
            self.regions
                .iter()
                .flat_map(|region| region.outer.points().iter().copied()),
        )
    }

    /// The area turned and moved by `transform`. A transform never mirrors, so the area
    /// stays clean without running the polygon library again.
    pub fn transformed(&self, transform: Transform) -> Self {
        let regions = self
            .regions
            .iter()
            .map(|region| Region {
                outer: region.outer.transformed(transform),
                holes: region
                    .holes
                    .iter()
                    .map(|hole| hole.transformed(transform))
                    .collect(),
            })
            .collect();
        Self { regions }
    }

    /// Grows the area by `distance_mm` on every side, or shrinks it when negative.
    ///
    /// This is the exact offset: every point of the new boundary is `distance_mm` from
    /// the old one. Corners that stick out get round, the way clearance around a product
    /// is round; corners that point in stay sharp. Rounding follows
    /// [`CURVE_TOLERANCE_MM`]. Shrinking can split a region at a narrow neck or remove it.
    pub fn offset(&self, distance_mm: f64) -> Result<Self, GeometryError> {
        if !distance_mm.is_finite() {
            return Err(GeometryError::OutOfRange);
        }
        if distance_mm == 0.0 || self.is_empty() {
            return Ok(self.clone());
        }
        let corner_step = chord_angle(distance_mm.abs(), CURVE_TOLERANCE_MM);
        let style = OutlineStyle::new(distance_mm).line_join(LineJoin::Round(corner_step));
        let shapes = self
            .to_shapes()
            .outline_fixed_scale(&style, GRID_PER_MM)
            .map_err(|_| GeometryError::OutOfRange)?;
        Ok(Self::from_shapes(shapes))
    }

    /// Everything covered by this area or `other`.
    pub fn union(&self, other: &Self) -> Result<Self, GeometryError> {
        self.combine(other, OverlayRule::Union)
    }

    /// Only what this area and `other` both cover.
    pub fn intersection(&self, other: &Self) -> Result<Self, GeometryError> {
        self.combine(other, OverlayRule::Intersect)
    }

    /// This area with `other` cut away.
    pub fn difference(&self, other: &Self) -> Result<Self, GeometryError> {
        self.combine(other, OverlayRule::Difference)
    }

    fn combine(&self, other: &Self, rule: OverlayRule) -> Result<Self, GeometryError> {
        overlay((&self.to_shapes(), &other.to_shapes()), rule)
    }

    fn to_shapes(&self) -> Shapes {
        self.regions
            .iter()
            .map(|region| {
                iter::once(&region.outer)
                    .chain(&region.holes)
                    .map(|ring| ring.0.clone())
                    .collect()
            })
            .collect()
    }

    fn from_shapes(shapes: Shapes) -> Self {
        let regions = shapes
            .into_iter()
            .filter_map(|shape| {
                let mut rings = shape.into_iter().map(Polygon);
                let outer = rings.next()?;
                Some(Region {
                    outer,
                    holes: rings.collect(),
                })
            })
            .collect();
        Self { regions }
    }
}

/// Runs one boolean on (subject, clip) on the fixed grid. Non-zero filling makes
/// overlapping rings that run the same way add up instead of cancelling.
fn overlay((subject, clip): (&Shapes, &Shapes), rule: OverlayRule) -> Result<Area, GeometryError> {
    let rings = || subject.iter().chain(clip).flatten();
    let adapter = grid_adapter(rings().flatten().copied())?;
    let capacity = rings().map(Vec::len).sum();
    let shapes = FloatOverlay::with_adapter(adapter, capacity)
        .unsafe_add_source(subject, ShapeType::Subject)
        .unsafe_add_source(clip, ShapeType::Clip)
        .overlay(rule, FillRule::NonZero);
    Ok(Area::from_shapes(shapes))
}

/// The 0.1 µm grid for one boolean, round all its `points`, with its origin on a whole
/// millimetre. The library would centre the grid on the points, so two operations could
/// snap one coordinate to neighbouring grid lines. Two panels built apart would then
/// meet with a 0.1 µm gap, and a box would fall apart at some sizes.
fn grid_adapter(
    points: impl Iterator<Item = Point> + Clone,
) -> Result<FloatPointAdapter<Point, i32>, GeometryError> {
    if !points.clone().all(Point::is_finite) {
        return Err(GeometryError::OutOfRange);
    }
    let bounds = Bounds::of(points).unwrap_or(Bounds {
        min: Point::ORIGIN,
        max: Point::ORIGIN,
    });
    let center = bounds.center();
    let (x, y) = (center.x.round(), center.y.round());
    // Whole millimetres both sides of (x, y), so the library finds that centre exactly;
    // the extra millimetre covers the rounding.
    let reach = (bounds.length_mm().max(bounds.width_mm()) / 2.0).ceil() + 1.0;
    let rect = FloatRect::new(x - reach, x + reach, y - reach, y + reach)
        .map_err(|_| GeometryError::OutOfRange)?;
    FloatPointAdapter::try_with_scale_conservative(rect, GRID_PER_MM)
        .map_err(|_| GeometryError::OutOfRange)
}

/// The polygon's points running clockwise on screen, the way outer rings run.
fn clockwise_points(polygon: &Polygon) -> Vec<Point> {
    let mut points = polygon.0.clone();
    if polygon.signed_area_mm2() < 0.0 {
        points.reverse();
    }
    points
}
