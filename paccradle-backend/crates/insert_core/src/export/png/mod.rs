//! The free watermarked PNG preview (M1-B5, CLAUDE.md §3 step 10, §7.4): one picture of a
//! design to keep or show a client.
//!
//! - The sizes and the fit in words, a top view of the products in the insert and box
//!   coloured by fit, and every piece flat (cut lines solid, creases dashed, folds
//!   numbered in order), with a legend.
//! - Drawn on the server with tiny-skia, in the site's colours and its typeface, Geist
//!   (bundled: `fonts/`).
//! - Not a production file (§7.4): it draws the preview's geometry
//!   ([`Packaging::rounded_for_preview`](crate::Packaging::rounded_for_preview)) as
//!   pixels, box pieces mirrored print side up like the production files, and a
//!   see-through watermark runs across the whole picture, over the drawings.

mod canvas;
mod draw;
mod font;
mod layout;

pub use layout::{Block, BlockKind, LegendItem, LegendMark, Line, Preview};

use thiserror::Error;

use crate::error::GeometryError;

/// Why a preview picture couldn't be made.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum PngError {
    /// The fit check couldn't rebuild a product's outline.
    #[error("fit check: {0}")]
    Fit(#[from] GeometryError),
    #[error("a {width} x {height} px picture can't be made")]
    Size { width: u32, height: u32 },
    #[error("the bundled font couldn't be read")]
    Font,
    #[error("PNG encoding failed: {0}")]
    Encode(String),
}
