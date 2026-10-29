//! What the engine produces: flat pieces to cut, with panels and hinges for the 3D fold
//! (CLAUDE.md §6.1, §7.2). Pieces are always generated, never sent in by users.
//!
//! Folding model: panels turn about their creases on the board's middle plane, so the
//! flat length between two creases is the middle-plane length of the folded panel. A
//! panel that ends up resting on a surface has its middle plane half a board above it.
//! A flap that folds round another layer of board keeps a strip next to its crease in
//! its parent's plane and turns past it ([`Hinge::offset_mm`]), the way its bend rolls
//! round that layer.

use serde::{Deserialize, Serialize};

use crate::geometry::{Line, Path, Polygon};

/// One flat part to cut, in mm.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Piece {
    /// Short label for files and the editor, e.g. "long divider".
    pub name: String,
    /// How many copies to cut.
    pub quantity: u32,
    /// Outer cut line (closed).
    pub outline: Path,
    /// Inner cut lines (closed): product holes, finger holes.
    pub holes: Vec<Path>,
    pub creases: Vec<Path>,
    pub perforations: Vec<Path>,
    /// Flat faces between the creases. Panel 0 is the root of the fold (the deck or
    /// box bottom). A piece that doesn't fold has one panel.
    pub panels: Vec<Panel>,
    /// Creases that fold, joining the panels into a tree rooted at panel 0.
    pub hinges: Vec<Hinge>,
}

/// One flat face of a piece, for the 3D preview.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Panel {
    pub outline: Polygon,
    pub holes: Vec<Polygon>,
}

/// A crease that folds `child` against `parent` (CLAUDE.md §7.2).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Hinge {
    /// Index into [`Piece::panels`], on the root side of the fold.
    pub parent: usize,
    /// Index into [`Piece::panels`] of the panel that turns.
    pub child: usize,
    /// The crease line the child turns around.
    pub axis: Line,
    /// Angle when fully folded, 0–180.
    pub fold_deg: f64,
    pub direction: FoldDirection,
    /// How far past the crease the child turns, in mm. The strip of the child between the
    /// crease and that line is the bend: it stays in the parent's plane, and the rest of
    /// the child turns about the line. 0 for most creases. A shipping box's outer flaps
    /// bend a board past their score, so they pass under the inner flaps while the score
    /// lines stay straight.
    pub offset_mm: f64,
    /// Fold order for the animation: lower stages fold first (walls before tabs).
    pub stage: u32,
}

/// Which way a crease folds, seen from the drawing's face: the side a viewer of the flat
/// drawing sees (x right, y down the page), as if it lay face up on a table. That's the
/// top of a card or a cradle deck, but the inside of an outer box, so a box's print side
/// is the back of its drawing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FoldDirection {
    /// The panels rise towards the viewer.
    Valley,
    /// The panels drop away from the viewer.
    Mountain,
}

impl FoldDirection {
    /// The same fold seen from the other face of the board.
    pub const fn flipped(self) -> Self {
        match self {
            FoldDirection::Valley => FoldDirection::Mountain,
            FoldDirection::Mountain => FoldDirection::Valley,
        }
    }
}
