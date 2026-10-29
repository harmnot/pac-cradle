//! Production files (CLAUDE.md §6.6, M1-B), made only on the server.
//!
//! Behind the `export` feature: the API turns it on, and the browser build
//! (`insert_core_wasm`) never does, so production files can't be made in the browser
//! (§7.4).
//!
//! Every file draws a [`Sheet`]: each piece once, laid out in rows, with box pieces
//! mirrored so their print side faces up. Cut files (SVG, DXF) use [`Spacing::CUT_FILE`];
//! drawings people read (PDF, with labels and dimensions) use [`Spacing::DRAWING`].
//! Writers: SVG (B1), PDF (B2), DXF (B3).
//!
//! Two free files aren't production files and draw the preview's geometry under a
//! watermark: the print-at-home test print (B4, [`TestPrint`]), every copy of every piece
//! tiled over A4 or Letter pages with fold steps; and the PNG preview (B5, [`Preview`]),
//! a picture of the pieces and of the products in the insert, coloured by fit.

mod dxf;
mod marks;
mod pdf;
mod png;
mod sheet;
mod svg;
mod test_print;

pub use png::{Block, BlockKind, LegendItem, LegendMark, Line, PngError, Preview};
pub use sheet::{Fold, LineKind, Sheet, SheetPiece, Spacing};
pub use test_print::{Joins, Paper, Polyline, TestItem, TestPage, TestPrint, Tile, View};

use crate::design::Design;
use crate::packaging::Packaging;

/// Decimals written for every coordinate: 0.1 µm, the geometry library's own grid.
const DECIMALS: usize = 4;

/// A number to [`DECIMALS`] places, without trailing zeros, and never "-0".
fn num(value: f64) -> String {
    let text = format!("{value:.DECIMALS$}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    match text {
        "-0" | "" => "0".to_owned(),
        _ => text.to_owned(),
    }
}

/// A share from 0 to 1 that looks random but comes out the same on every run, for row
/// `row` of a watermark seeded by `seed` (a SplitMix64 hash). Shifts each row along by
/// its own amount.
fn jitter((seed, row): (u64, i64)) -> f64 {
    let mut hash =
        seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (row as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    hash ^= hash >> 31;
    hash = hash.wrapping_mul(0x94D0_49BB_1331_11EB);
    hash ^= hash >> 29;
    (hash % 1000) as f64 / 1000.0
}

impl Packaging {
    /// The SVG production file for this packaging (see [`Sheet::to_svg`]).
    pub fn to_svg(&self) -> String {
        Sheet::of((self, Spacing::CUT_FILE)).to_svg()
    }

    /// The PDF production file for this packaging, with labels and dimensions (see
    /// [`Sheet::to_pdf`]).
    pub fn to_pdf(&self) -> Vec<u8> {
        Sheet::of((self, Spacing::DRAWING)).to_pdf()
    }

    /// The DXF production file for this packaging (see [`Sheet::to_dxf`]).
    pub fn to_dxf(&self) -> String {
        Sheet::of((self, Spacing::CUT_FILE)).to_dxf()
    }

    /// The free print-at-home test PDF for this packaging on `paper` (see
    /// [`TestPrint::to_pdf`]).
    pub fn to_test_print(&self, paper: Paper) -> Vec<u8> {
        TestPrint::of((self, paper)).to_pdf()
    }

    /// The free watermarked PNG preview of this packaging, generated from `design` (see
    /// [`Preview::to_png`]).
    pub fn to_png(&self, design: &Design) -> Result<Vec<u8>, PngError> {
        Preview::of((self, design))?.to_png()
    }
}
