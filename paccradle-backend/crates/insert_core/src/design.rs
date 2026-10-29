//! What the user asks for: products, insert style, board and outer box (CLAUDE.md §6.1–6.3).
//!
//! A [`Design`] is stored as `designs.spec` (jsonb) and sent to the browser preview.
//! It comes from users, so call [`Design::validate`] before generating anything.

use serde::{Deserialize, Serialize};

use crate::geometry::{Point, Polygon};
use crate::material::Material;

/// Everything needed to generate one insert and its outer box.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Design {
    /// Board for the insert. The outer box has its own (see [`BoxSpec`]).
    pub material: Material,
    pub products: Vec<Product>,
    pub insert: InsertSpec,
    /// Box sized around the insert. `None` designs the insert only.
    pub outer_box: Option<BoxSpec>,
}

/// One product type the insert holds, from a capture or typed sizes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Product {
    /// Unique within the design. Placements refer to it.
    pub id: String,
    /// Top-view outline, in mm from the outline's top-left corner (as `contour_mm` in the vision output).
    pub contour_mm: Polygon,
    pub height_mm: f64,
    /// How many of this product the insert holds.
    pub quantity: u32,
}

/// The insert style and its settings (CLAUDE.md §6.2). The MVP has three styles (DEC-08).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "style", rename_all = "snake_case")]
pub enum InsertSpec {
    /// Slotted dividers for several identical items (FEFCO 09xx interior fittings).
    PartitionGrid(PartitionGridSpec),
    /// Flat card with product-shaped holes.
    CutoutCard(CutoutCardSpec),
    /// Raised deck with holes; walls fold down and lock with tabs.
    CradlePlatform(CradlePlatformSpec),
}

/// Partition grid settings. Slot width and depth come from the board (CLAUDE.md §6.2).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PartitionGridSpec {
    /// Cells across y (the box's width).
    pub rows: u32,
    /// Cells along x (the box's length).
    pub columns: u32,
    pub divider_height_mm: f64,
    /// Adds strips round the outside of the grid.
    pub outer_frame: bool,
    /// Half-round notches in the strips' top edges, one in the middle of each cell wall.
    pub finger_notches: bool,
}

/// Cutout card settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CutoutCardSpec {
    /// Smallest distance from a hole to the card edge.
    pub margin_mm: f64,
    /// Half-round notches beside each hole, to get a finger in. `None` cuts plain holes.
    #[serde(default)]
    pub finger_notches: Option<FingerNotches>,
    /// Where the products the generator lays out sit on a card of fixed size (a box with
    /// a fixed inside size). Centred by default. Rows inside the block line up the same way.
    #[serde(default)]
    pub align: Align,
    /// Where each product sits. Products without one are laid out by the generator.
    #[serde(default)]
    pub placements: Vec<Placement>,
}

/// Where the finger notch sits on each hole. Sides are named as the product lies
/// lengthwise (long side along x, the way the generator lays it); they turn with the
/// product. Every field can be left out of the JSON.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FingerNotches {
    /// Which long side gets a notch.
    pub side: NotchSide,
    /// Where along that side the notch's centre sits: 0 at the left end, 0.5 in the
    /// middle, 1 at the right end.
    pub along: f64,
}

impl Default for FingerNotches {
    /// One notch in the middle of the bottom long side.
    fn default() -> Self {
        Self {
            side: NotchSide::Bottom,
            along: 0.5,
        }
    }
}

/// Which long side of a hole gets a finger notch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotchSide {
    /// The side with the larger y (lower on screen).
    #[default]
    Bottom,
    Top,
    /// One notch on each long side, to pinch the product out.
    Both,
}

/// Where a block of products sits in the space it may fill.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Align {
    TopLeft,
    Top,
    TopRight,
    Left,
    #[default]
    Center,
    Right,
    BottomLeft,
    Bottom,
    BottomRight,
}

impl Align {
    /// How far into the spare room the block moves, along x and along y: 0 keeps it at
    /// the left or top, 0.5 centres it, 1 pushes it to the right or bottom.
    pub const fn shares(self) -> (f64, f64) {
        match self {
            Self::TopLeft => (0.0, 0.0),
            Self::Top => (0.5, 0.0),
            Self::TopRight => (1.0, 0.0),
            Self::Left => (0.0, 0.5),
            Self::Center => (0.5, 0.5),
            Self::Right => (1.0, 0.5),
            Self::BottomLeft => (0.0, 1.0),
            Self::Bottom => (0.5, 1.0),
            Self::BottomRight => (1.0, 1.0),
        }
    }
}

/// Cradle platform settings. Leg and lock tab sizes follow from the deck height and
/// the board.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CradlePlatformSpec {
    /// Height of the deck's top above the box floor.
    pub deck_height_mm: f64,
    /// Smallest distance from a hole to the deck edge. Along the long edges, the holes
    /// also keep clear of the legs and their lock slots, which can take more.
    pub margin_mm: f64,
    /// Where the products the generator lays out sit on a deck of fixed size (a box with
    /// a fixed inside size). Centred by default, as on the cutout card.
    #[serde(default)]
    pub align: Align,
    /// Where each product sits. Products without one are laid out by the generator.
    #[serde(default)]
    pub placements: Vec<Placement>,
}

/// Where one product sits on a card or deck, set by dragging in the editor (CLAUDE.md §7.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Placement {
    pub product_id: String,
    /// Where the centre of the outline's bounding box (taken before turning) lands, in mm
    /// from the insert's top-left corner.
    pub center_mm: Point,
    /// Turn of the outline around that centre; positive is clockwise on screen.
    pub rotation_deg: f64,
}

/// Outer box styles in the MVP (CLAUDE.md §6.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BoxStyle {
    /// FEFCO 0427 style: hinged lid with locking tabs.
    Mailer,
    /// FEFCO 0201: regular slotted container.
    Shipping,
}

/// The outer box.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BoxSpec {
    pub style: BoxStyle,
    /// Board for the box, often corrugated even when the insert is paperboard.
    pub material: Material,
    /// A fixed inside size, e.g. a box the brand already stocks. `None` sizes the box
    /// around the insert (inside = insert + clearance).
    pub inside_mm: Option<BoxSize>,
}

/// Inside length × width × height of a box.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BoxSize {
    pub length_mm: f64,
    pub width_mm: f64,
    pub height_mm: f64,
}
