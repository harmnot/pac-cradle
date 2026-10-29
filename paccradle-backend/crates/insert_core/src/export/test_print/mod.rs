//! Print-at-home test print (M1-B4, CLAUDE.md U3 and §6.6): a free paper prototype of
//! the insert and its box, printed on A4 or US Letter and built in about 10 minutes.
//!
//! - Every copy of every piece, at true size, so the whole set can be built at once. A
//!   piece that fits on a page prints whole; bigger ones are tiled over pages that
//!   overlap ([`TestPrint::of`]).
//! - Tiled pages carry a hatched strip to cut off, a line to lay the next page's edge on,
//!   grey diamonds that join up across the seam, and the neighbouring tile's name. A tile
//!   the piece doesn't reach (a box's corner) isn't printed.
//! - Each fold's step number sits in a circle on its crease: white folds away from the
//!   printed side, black towards it. Box pieces print mirrored, print side up, like the
//!   production files, so their folds all go away from the printed side.
//! - The first page says how to build it, and every page has a scale bar to check that
//!   it printed at 100 %.
//! - Not a production file (CLAUDE.md §7.4): it draws the preview's geometry (straight
//!   segments, [`Packaging::rounded_for_preview`](crate::Packaging::rounded_for_preview)),
//!   with no layers, each tiled page holds only its own part of a piece, and a watermark
//!   runs across every page, over the lines.

mod clip;
mod instructions;
mod layout;
mod pages;

use crate::check::Part;
use crate::export::{Fold, LineKind};
use crate::geometry::{Bounds, Point};

/// Unprinted edge left on each side of the paper: home printers can't print to the edge,
/// and 10 mm clears nearly all of them.
const MARGIN_MM: f64 = 10.0;

/// The band at the foot of each page, under the drawing: page number and scale bar.
const FOOTER_MM: f64 = 10.0;

/// How far neighbouring tiles overlap. The seam runs down the middle of the overlap.
const OVERLAP_MM: f64 = 10.0;

/// Space between whole pieces sharing a page, to cut between.
const GAP_MM: f64 = 6.0;

/// Room above a whole piece for its label.
const LABEL_MM: f64 = 6.0;

/// Space kept round a tiled piece, so its outline never runs along a page's edge.
const TILE_PAD_MM: f64 = 3.0;

/// Home printer paper, used upright.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Paper {
    /// 210 × 297 mm.
    A4,
    /// US Letter, 8.5 × 11 in.
    Letter,
}

impl Paper {
    /// Width and height in mm, upright.
    pub const fn size_mm(self) -> (f64, f64) {
        match self {
            Paper::A4 => (210.0, 297.0),
            Paper::Letter => (215.9, 279.4),
        }
    }

    /// Where the drawing goes on each page, in page mm (y down): inside the printer
    /// margin and above the footer.
    pub fn window(self) -> Bounds {
        let (width, height) = self.size_mm();
        Bounds {
            min: Point::new(MARGIN_MM, MARGIN_MM),
            max: Point::new(width - MARGIN_MM, height - MARGIN_MM - FOOTER_MM),
        }
    }

    /// The scale check at the foot of each page: 50 mm on A4, 2 in on Letter, where
    /// rulers are in inches.
    fn scale_bar(self) -> ScaleBar {
        match self {
            Paper::A4 => ScaleBar {
                length_mm: 50.0,
                parts: 5,
                label: "50 mm",
            },
            Paper::Letter => ScaleBar {
                length_mm: 50.8,
                parts: 4,
                label: "2 in",
            },
        }
    }
}

/// A bar of known length, split into equal parts.
#[derive(Debug, Clone, Copy)]
struct ScaleBar {
    length_mm: f64,
    parts: u32,
    label: &'static str,
}

/// The test print laid out: what each page shows. [`TestPrint::to_pdf`] draws it.
#[derive(Debug, Clone, PartialEq)]
pub struct TestPrint {
    pub paper: Paper,
    /// Every copy of every piece: the production sheet's pieces in order (inserts first,
    /// box pieces mirrored), each copy after the one before.
    pub items: Vec<TestItem>,
    /// The drawing pages, after the instructions: pages of whole pieces first, then each
    /// tiled piece's pages, row by row.
    pub pages: Vec<TestPage>,
    /// True when the insert has strips that slot together (a partition grid).
    pub slotted_strips: bool,
}

/// One copy of one piece, in its own coordinates: top-left corner at (0, 0), x right,
/// y down, turned a quarter turn clockwise from the sheet when that fits the paper
/// better.
#[derive(Debug, Clone, PartialEq)]
pub struct TestItem {
    pub part: Part,
    /// What the piece is, e.g. "Box: mailer" ([`SheetPiece::label`](crate::export::SheetPiece::label)).
    pub piece_label: String,
    /// Which copy this is, from 1.
    pub copy: u32,
    /// How many copies of the piece there are.
    pub copies: u32,
    pub turned: bool,
    pub length_mm: f64,
    pub width_mm: f64,
    pub cuts: Vec<Polyline>,
    pub creases: Vec<Polyline>,
    pub perforations: Vec<Polyline>,
    pub folds: Vec<Fold>,
}

impl TestItem {
    /// What it is and which copy: "Insert: long divider (1 of 2)", or "Box: mailer" for a
    /// piece cut once.
    pub fn label(&self) -> String {
        match self.copies {
            0 | 1 => self.piece_label.clone(),
            copies => format!("{} ({} of {copies})", self.piece_label, self.copy),
        }
    }

    /// The item's lines of one kind.
    pub fn lines(&self, kind: LineKind) -> &[Polyline] {
        match kind {
            LineKind::Cut => &self.cuts,
            LineKind::Crease => &self.creases,
            LineKind::Perforation => &self.perforations,
        }
    }
}

/// A drawn line as straight segments between its points.
#[derive(Debug, Clone, PartialEq)]
pub struct Polyline {
    pub points: Vec<Point>,
    /// True when the last point joins back to the first.
    pub closed: bool,
}

/// One drawing page.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TestPage {
    /// What it shows: each whole piece on it, or one tile of a bigger piece.
    pub views: Vec<View>,
    /// Which tile of its piece the page is; `None` on a page of whole pieces.
    pub tile: Option<Tile>,
}

/// Part of one item, drawn on a page.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    /// Index into [`TestPrint::items`].
    pub item: usize,
    /// The part of the item shown, in the item's coordinates. Lines are cut off at its
    /// edges.
    pub window: Bounds,
    /// Where the window's top-left corner lands on the page, in page mm (y down).
    pub at: Point,
}

impl View {
    /// Where a point of the item lands on the page.
    pub fn to_page(&self, point: Point) -> Point {
        Point::new(
            self.at.x + point.x - self.window.min.x,
            self.at.y + point.y - self.window.min.y,
        )
    }
}

/// One page of a piece tiled over several, counted from the top-left. A tile the piece
/// doesn't reach isn't printed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tile {
    pub column: usize,
    pub row: usize,
    pub columns: usize,
    pub rows: usize,
    /// Which sides meet another printed tile of the piece.
    pub joins: Joins,
}

/// Which sides of a tile meet a neighbouring printed tile.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Joins {
    pub left: bool,
    pub right: bool,
    pub top: bool,
    pub bottom: bool,
}

impl Tile {
    /// Its name: the row's letter and the column's number, "A1" at the top-left, "B3" in
    /// the second row's third column.
    pub fn name(self) -> String {
        tile_name((self.column, self.row))
    }
}

/// The name of the tile at `(column, row)`. Past row Z, rows are numbered: "R27-1".
fn tile_name((column, row): (usize, usize)) -> String {
    let row_name = u8::try_from(row).ok().filter(|row| *row < 26).map_or_else(
        || format!("R{}-", row + 1),
        |row| char::from(b'A' + row).to_string(),
    );
    format!("{row_name}{}", column + 1)
}

/// True when `point` lies inside `bounds`, edges included.
fn inside((point, bounds): (Point, Bounds)) -> bool {
    (bounds.min.x..=bounds.max.x).contains(&point.x)
        && (bounds.min.y..=bounds.max.y).contains(&point.y)
}
