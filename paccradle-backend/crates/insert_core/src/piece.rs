//! What the engine produces: flat pieces to cut, with panels and hinges for the 3D fold
//! (CLAUDE.md §6.1, §7.2). Pieces are always generated, never sent in by users.

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
    /// Fold order for the animation: lower stages fold first (walls before tabs).
    pub stage: u32,
}

/// Which way a crease folds, seen from the top (printed) side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FoldDirection {
    /// The panels rise towards the viewer.
    Valley,
    /// The panels drop away from the viewer.
    Mountain,
}
