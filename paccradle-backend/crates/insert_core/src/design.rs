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
    pub rows: u32,
    pub columns: u32,
    pub divider_height_mm: f64,
    /// Adds a frame around the grid.
    pub outer_frame: bool,
    pub finger_notches: bool,
}

/// Cutout card settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CutoutCardSpec {
    /// Smallest distance from a hole to the card edge.
    pub margin_mm: f64,
    pub finger_notches: bool,
    /// Where each product sits. Empty means the generator lays them out (M1-A3).
    #[serde(default)]
    pub placements: Vec<Placement>,
}

/// Cradle platform settings. Wall and tab sizes follow from the deck height (M1-A5).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CradlePlatformSpec {
    /// Height of the deck above the box floor.
    pub deck_height_mm: f64,
    /// Smallest distance from a hole to the deck edge.
    pub margin_mm: f64,
    /// Where each product sits. Empty means the generator lays them out.
    #[serde(default)]
    pub placements: Vec<Placement>,
}

/// Where one product sits on a card or deck, set by dragging in the editor (CLAUDE.md §7.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Placement {
    pub product_id: String,
    /// Centre of the product's outline, in mm from the insert's top-left corner.
    pub center_mm: Point,
    /// Turn of the outline around its centre; positive is clockwise on screen.
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
