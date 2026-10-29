//! Insert generators (CLAUDE.md §6.2): a [`Design`] in, the [`Insert`] to cut out.
//!
//! The cutout card (M1-A3), the partition grid (A4) and the cradle platform (A5). Each
//! plugs into [`Design::generate_insert`] through `GenerateInsert`.

mod cradle_platform;
mod cutout_card;
mod hole;
mod layout;
mod partition_grid;

use serde::{Deserialize, Serialize};

use crate::design::{BoxSpec, Design, InsertSpec, Placement};
use crate::error::{GenerateError, GeometryError};
use crate::geometry::{Area, Polygon, Region};
use crate::material::Material;
use crate::piece::{Panel, Piece};

use hole::{HoleStyle, ProductHole};

pub use hole::FINGER_NOTCH_RADIUS_MM;

/// A generated insert: the pieces to cut, its size in the box, and where each product sits.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Insert {
    pub pieces: Vec<Piece>,
    /// Footprint along x, as the insert lies in the box.
    pub length_mm: f64,
    /// Footprint along y.
    pub width_mm: f64,
    /// How tall the insert stands in the box: the board thickness for a flat card, the
    /// divider height for a grid, the deck's top for a cradle.
    pub height_mm: f64,
    /// One placement for every product copy, in the insert's frame (top-left corner at
    /// 0, 0): the design's own placements first, then the ones the generator chose. Send
    /// them back as the design's placements to keep this layout while editing.
    pub placements: Vec<Placement>,
    /// Where every cut copy of every piece goes in the assembled insert, for the 3D
    /// preview: one entry per copy, so a piece with quantity 3 appears 3 times.
    pub mounts: Vec<Mount>,
}

/// Where one cut copy of a piece goes in the assembled insert.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Mount {
    /// Index into [`Insert::pieces`].
    pub piece: usize,
    pub stance: Stance,
    /// Where the piece's (0, 0) lands, in mm: x and y in the insert's frame, z up from
    /// the box floor. For a piece standing upright, this is on its middle plane; for a
    /// flat one, on its underside. A piece that folds is placed by its root panel (panel
    /// 0); the other panels fold from it along the hinges.
    pub at_mm: [f64; 3],
}

/// How a flat piece sits in the assembled insert or box. A point (u, v) of the flat piece,
/// with v pointing down the drawing, lands at `at_mm` plus the offset each variant gives.
/// The drawing's face (the side its viewer sees, see [`FoldDirection`]) looks the way
/// each variant says.
///
/// [`FoldDirection`]: crate::FoldDirection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stance {
    /// Lying flat: (u, v, 0), the face looking up (+z). Cards, the deck of a cradle and
    /// the base of a mailer box.
    Flat,
    /// Standing in a plane along x: (u, 0, −v), the face looking along +y. Grid strips
    /// that run along the length, and the back wall of a shipping box, facing in.
    UprightAlongX,
    /// Standing in a plane along y: (0, u, −v), the face looking along −x. Grid strips
    /// that run across.
    UprightAlongY,
}

/// An insert style that builds itself from a design (M1-A3 to A5).
trait GenerateInsert {
    /// Builds the insert. `design` has passed [`Design::validate`], and `self` is its
    /// `insert` settings.
    fn generate(&self, design: &Design) -> Result<Insert, GenerateError>;
}

impl Design {
    /// Checks the design, then generates its insert.
    pub fn generate_insert(&self) -> Result<Insert, GenerateError> {
        self.validate()?;
        match &self.insert {
            InsertSpec::CutoutCard(card) => card.generate(self),
            InsertSpec::PartitionGrid(grid) => grid.generate(self),
            InsertSpec::CradlePlatform(cradle) => cradle.generate(self),
        }
    }
}

/// One product copy where the insert holds it, in the insert's frame.
pub(crate) struct PlacedProduct {
    /// Index into [`Insert::placements`].
    pub placement: usize,
    /// Index into the design's products.
    pub product: usize,
    /// The product's own outline: no clearance, no notches.
    pub outline: Polygon,
    /// The hole the insert cuts for it; `None` when the style cuts none (a partition
    /// grid holds products in cells).
    pub hole: Option<Area>,
}

impl InsertSpec {
    /// How this style cuts product holes, or `None` when it cuts none.
    fn cut_hole_style(&self, material: &Material) -> Option<HoleStyle> {
        match self {
            Self::CutoutCard(card) => Some(card.hole_style(material)),
            Self::CradlePlatform(cradle) => Some(cradle.hole_style(material)),
            Self::PartitionGrid(_) => None,
        }
    }
}

/// Every product copy in `insert`, generated from `design`, in placement order: its
/// outline and the hole cut for it, built the way the generator builds them. Placements
/// of products the design doesn't have are left out.
pub(crate) fn placed_products(
    (design, insert): (&Design, &Insert),
) -> Result<Vec<PlacedProduct>, GeometryError> {
    let cut = design.insert.cut_hole_style(&design.material);
    // A grid cuts no holes, but the outline comes from the same straightened shape.
    let style = cut.unwrap_or(HoleStyle {
        clearance_mm: design.material.clearance_mm,
        finger_notches: None,
    });
    let holes = design
        .products
        .iter()
        .map(|product| ProductHole::new(product, style))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(insert
        .placements
        .iter()
        .enumerate()
        .filter_map(|(placement, at)| {
            let product = design
                .products
                .iter()
                .position(|product| product.id == at.product_id)?;
            let hole = &holes[product];
            Some(PlacedProduct {
                placement,
                product,
                outline: hole.placed_outline(at),
                hole: cut.map(|_| hole.placed(at)),
            })
        })
        .collect())
}

/// The room a box with a fixed inside size leaves for the insert: that size minus the
/// box's clearance on each side. Length runs along x. `None` when the box is sized
/// around the insert, or there's no box.
fn fixed_room(outer_box: Option<&BoxSpec>) -> Option<(f64, f64)> {
    let outer_box = outer_box?;
    let inside = outer_box.inside_mm?;
    let gaps = 2.0 * outer_box.material.clearance_mm;
    Some((inside.length_mm - gaps, inside.width_mm - gaps))
}

/// A flat piece that doesn't fold, cut along the region's rings. `None` when the region
/// has no outline.
fn flat_piece((name, region): (&str, Region)) -> Option<Piece> {
    let outline = region.outer.to_path()?;
    Some(Piece {
        name: name.to_owned(),
        quantity: 1,
        outline,
        holes: region.holes.iter().filter_map(Polygon::to_path).collect(),
        creases: Vec::new(),
        perforations: Vec::new(),
        panels: vec![Panel {
            outline: region.outer,
            holes: region.holes,
        }],
        hinges: Vec::new(),
    })
}
