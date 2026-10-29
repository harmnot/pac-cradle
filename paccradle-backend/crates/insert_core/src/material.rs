//! Boards and their starting tolerances (CLAUDE.md §6.4).
//!
//! The numbers are common ranges and starting values, not measured ones. Users can
//! edit every value, and the app tells them to check with their printer.

use serde::{Deserialize, Serialize};

/// Board families the generators know.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Board {
    /// Folding boxboard / SBS.
    Paperboard,
    /// Chipboard / greyboard.
    Chipboard,
    /// Corrugated, E-flute.
    EFlute,
    /// Corrugated, B-flute.
    BFlute,
    /// Corrugated, C-flute.
    CFlute,
}

/// Narrowest strip of board CLAUDE.md §6.5 accepts, whatever the thickness.
const MIN_WEB_MM: f64 = 3.0;

/// A closed range of millimetres.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MmRange {
    pub min: f64,
    pub max: f64,
}

impl MmRange {
    /// True when `value_mm` lies inside the range, ends included.
    pub fn contains(self, value_mm: f64) -> bool {
        (self.min..=self.max).contains(&value_mm)
    }
}

impl Board {
    /// Every board, in the order the editor lists them.
    pub const ALL: [Self; 5] = [
        Self::Paperboard,
        Self::Chipboard,
        Self::EFlute,
        Self::BFlute,
        Self::CFlute,
    ];

    /// Typical thickness for this board (CLAUDE.md §6.4). Outside it, the editor warns (M1-A7).
    pub const fn typical_thickness(self) -> MmRange {
        match self {
            Self::Paperboard => MmRange { min: 0.4, max: 0.6 },
            Self::Chipboard => MmRange { min: 1.0, max: 2.5 },
            Self::EFlute => MmRange { min: 1.2, max: 2.0 },
            Self::BFlute => MmRange { min: 2.5, max: 3.5 },
            Self::CFlute => MmRange { min: 3.5, max: 4.5 },
        }
    }

    /// True for corrugated board.
    pub const fn is_corrugated(self) -> bool {
        matches!(self, Self::EFlute | Self::BFlute | Self::CFlute)
    }
}

/// The board plus the numbers the generators use for it.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Material {
    pub board: Board,
    pub thickness_mm: f64,
    /// Gap between the product and the insert, per side.
    pub clearance_mm: f64,
    /// Added to the board thickness to get slot widths.
    pub slot_tolerance_mm: f64,
}

impl Material {
    /// Starting values for a board (CLAUDE.md §6.4): mid-range thickness, the table's
    /// clearance (the middle of B-flute's 1.5–2 mm) and slot tolerance. Verify with test cuts.
    pub const fn starting_values(board: Board) -> Self {
        let (thickness_mm, clearance_mm, slot_tolerance_mm) = match board {
            Board::Paperboard => (0.5, 1.0, 0.3),
            Board::Chipboard => (1.5, 1.0, 0.3),
            Board::EFlute => (1.6, 1.5, 0.5),
            Board::BFlute => (3.0, 1.75, 0.5),
            Board::CFlute => (4.0, 2.0, 0.5),
        };
        Self {
            board,
            thickness_mm,
            clearance_mm,
            slot_tolerance_mm,
        }
    }

    /// Narrowest strip of board to leave between two holes, or between a hole and an
    /// edge (CLAUDE.md §6.5): 3 mm or 3× the thickness, whichever is more.
    pub fn min_web_mm(&self) -> f64 {
        (3.0 * self.thickness_mm).max(MIN_WEB_MM)
    }
}
