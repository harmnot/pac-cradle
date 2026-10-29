//! Where each piece goes on the paper.
//!
//! - A piece that fits the page's drawing window, with room for its label, prints whole.
//!   Whole pieces share pages in rows, in order, each as drawn unless turning it a quarter
//!   turn is the only way it fits, or keeps it on the current row or page.
//! - A bigger piece gets pages of its own: windows that overlap by [`OVERLAP_MM`], as many
//!   as cover it plus [`TILE_PAD_MM`] round it, centred on it, and turned whichever way
//!   needs fewer pages. A window the piece doesn't reach (beside a box's arm) is left
//!   out.

use super::{
    GAP_MM, Joins, LABEL_MM, OVERLAP_MM, Paper, Polyline, TILE_PAD_MM, TestItem, TestPage,
    TestPrint, Tile, View,
};
use crate::export::{Fold, LineKind, Sheet, SheetPiece, Spacing};
use crate::geometry::{Bounds, CURVE_TOLERANCE_MM, Line, Path, Point, Polygon};
use crate::insert::Stance;
use crate::packaging::Packaging;

/// A tiny overshoot that doesn't earn a whole extra page.
const SLACK_MM: f64 = 1e-6;

/// How far a whole piece's window reaches past its outline, so no line sits on the edge.
const WHOLE_SLACK_MM: f64 = 0.5;

impl TestPrint {
    /// Lays out every copy of every piece of `packaging` on `paper`, drawn from the
    /// preview's geometry.
    pub fn of((packaging, paper): (&Packaging, Paper)) -> TestPrint {
        let preview = packaging.rounded_for_preview();
        let sheet = Sheet::of((&preview, Spacing::CUT_FILE));
        let window = paper.window();
        let mut items = Vec::new();
        let mut whole = Shelves::new(window);
        let mut tiled = Vec::new();
        for piece in &sheet.pieces {
            let fit = Fit::choose(((piece.bounds.length_mm(), piece.bounds.width_mm()), window));
            for copy in 1..=piece.quantity {
                let index = items.len();
                let item = match fit.tiling {
                    None => whole.add((index, piece, (copy, fit.turned))),
                    Some(tiling) => {
                        tiled.push((index, tiling));
                        TestItem::of((piece, (copy, fit.turned)))
                    }
                };
                items.push(item);
            }
        }
        let mut pages = whole.into_pages();
        for (index, tiling) in tiled {
            if let Some(item) = items.get(index) {
                pages.extend(tiling.pages(((index, item), window.min)));
            }
        }
        TestPrint {
            paper,
            items,
            pages,
            slotted_strips: packaging
                .insert
                .mounts
                .iter()
                .any(|mount| matches!(mount.stance, Stance::UprightAlongX | Stance::UprightAlongY)),
        }
    }
}

impl TestItem {
    /// Copy `copy` of a sheet piece, moved to its own coordinates and turned when `turned`.
    fn of((piece, (copy, turned)): (&SheetPiece, (u32, bool))) -> TestItem {
        let place = Place {
            origin: piece.bounds.min,
            width_mm: piece.bounds.width_mm(),
            turned,
        };
        let lines = |paths: &[Path]| paths.iter().map(|path| place.polyline(path)).collect();
        let (length_mm, width_mm) = match turned {
            false => (piece.bounds.length_mm(), piece.bounds.width_mm()),
            true => (piece.bounds.width_mm(), piece.bounds.length_mm()),
        };
        TestItem {
            part: piece.part,
            piece_label: piece.label(),
            copy,
            copies: piece.quantity,
            turned,
            length_mm,
            width_mm,
            cuts: lines(&piece.cuts),
            creases: lines(&piece.creases),
            perforations: lines(&piece.perforations),
            folds: piece
                .folds
                .iter()
                .map(|fold| Fold {
                    axis: Line {
                        from: place.point(fold.axis.from),
                        to: place.point(fold.axis.to),
                    },
                    ..*fold
                })
                .collect(),
        }
    }

    /// True when some of the piece lies in `window`: a line runs through it, or it lies
    /// inside the outline (a big panel).
    fn reaches(&self, window: Bounds) -> bool {
        let crossed = LineKind::ALL
            .iter()
            .flat_map(|&kind| self.lines(kind))
            .any(|line| !line.clipped(window).is_empty());
        crossed
            || self
                .cuts
                .first()
                .is_some_and(|outline| Polygon(outline.points.clone()).contains(window.center()))
    }
}

/// How a sheet piece's points land in an item's coordinates: the piece's top-left corner
/// at (0, 0), turned a quarter turn clockwise when `turned`. Turning never mirrors, so
/// box pieces keep their print side up.
struct Place {
    origin: Point,
    /// The piece's extent along y on the sheet.
    width_mm: f64,
    turned: bool,
}

impl Place {
    fn point(&self, point: Point) -> Point {
        let (x, y) = (point.x - self.origin.x, point.y - self.origin.y);
        match self.turned {
            false => Point::new(x, y),
            // The top edge turns to the right-hand side, exactly (no sines).
            true => Point::new(self.width_mm - y, x),
        }
    }

    fn polyline(&self, path: &Path) -> Polyline {
        Polyline {
            points: path
                .flatten(CURVE_TOLERANCE_MM)
                .into_iter()
                .map(|point| self.point(point))
                .collect(),
            closed: path.closed,
        }
    }
}

/// How one piece goes on the paper.
#[derive(Debug, Clone, Copy)]
struct Fit {
    turned: bool,
    /// `None` when the piece prints whole.
    tiling: Option<Tiling>,
}

impl Fit {
    /// Whole as drawn, else whole turned, else tiled whichever way takes fewer pages
    /// (as drawn on a tie).
    fn choose(((length_mm, width_mm), window): ((f64, f64), Bounds)) -> Fit {
        let room = (window.length_mm(), window.width_mm());
        let whole = |(length, width): (f64, f64)| length <= room.0 && width + LABEL_MM <= room.1;
        if whole((length_mm, width_mm)) {
            return Fit {
                turned: false,
                tiling: None,
            };
        }
        if whole((width_mm, length_mm)) {
            return Fit {
                turned: true,
                tiling: None,
            };
        }
        let straight = Tiling::over(((length_mm, width_mm), room));
        let turned = Tiling::over(((width_mm, length_mm), room));
        match turned.count() < straight.count() {
            true => Fit {
                turned: true,
                tiling: Some(turned),
            },
            false => Fit {
                turned: false,
                tiling: Some(straight),
            },
        }
    }
}

/// The overlapping windows that cover one piece, in the piece's coordinates.
#[derive(Debug, Clone, Copy)]
struct Tiling {
    columns: usize,
    rows: usize,
    /// The top-left tile's window corner.
    first: Point,
    /// The window's size: the page's drawing window.
    room: (f64, f64),
}

impl Tiling {
    /// The tiles over a piece `length_mm` × `width_mm` with windows of `room`.
    fn over(((length_mm, width_mm), room): ((f64, f64), (f64, f64))) -> Tiling {
        let (columns, first_x) = along((length_mm, room.0));
        let (rows, first_y) = along((width_mm, room.1));
        Tiling {
            columns,
            rows,
            first: Point::new(first_x, first_y),
            room,
        }
    }

    fn count(self) -> usize {
        self.columns * self.rows
    }

    /// One page per tile the item reaches, row by row, each showing item `item` with its
    /// window at the page's drawing window corner `at`.
    fn pages(self, ((index, item), at): ((usize, &TestItem), Point)) -> Vec<TestPage> {
        let window = |(column, row): (usize, usize)| {
            let min = Point::new(
                self.first.x + column as f64 * (self.room.0 - OVERLAP_MM),
                self.first.y + row as f64 * (self.room.1 - OVERLAP_MM),
            );
            Bounds {
                min,
                max: Point::new(min.x + self.room.0, min.y + self.room.1),
            }
        };
        let reached: Vec<Vec<bool>> = (0..self.rows)
            .map(|row| {
                (0..self.columns)
                    .map(|column| item.reaches(window((column, row))))
                    .collect()
            })
            .collect();
        let printed = |(column, row): (usize, usize)| {
            reached
                .get(row)
                .and_then(|tiles| tiles.get(column))
                .copied()
                .unwrap_or(false)
        };
        let mut pages = Vec::new();
        for row in 0..self.rows {
            for column in (0..self.columns).filter(|&column| printed((column, row))) {
                let joins = Joins {
                    left: column > 0 && printed((column - 1, row)),
                    right: printed((column + 1, row)),
                    top: row > 0 && printed((column, row - 1)),
                    bottom: printed((column, row + 1)),
                };
                pages.push(TestPage {
                    views: vec![View {
                        item: index,
                        window: window((column, row)),
                        at,
                    }],
                    tile: Some(Tile {
                        column,
                        row,
                        columns: self.columns,
                        rows: self.rows,
                        joins,
                    }),
                });
            }
        }
        pages
    }
}

/// How many windows `room` long, each overlapping the last by [`OVERLAP_MM`], cover
/// `length` plus [`TILE_PAD_MM`] at each end; and where the first starts, so the piece
/// sits in the middle of them.
fn along((length, room): (f64, f64)) -> (usize, f64) {
    let padded = length + 2.0 * TILE_PAD_MM;
    let step = room - OVERLAP_MM;
    let count = match padded <= room {
        true => 1,
        false => ((padded - room) / step - SLACK_MM).ceil() as usize + 1,
    };
    let covered = room + (count - 1) as f64 * step;
    (count, (length - covered) / 2.0)
}

/// Fills pages with whole pieces: left to right in rows, rows top to bottom, a new page
/// when the next row doesn't fit.
struct Shelves {
    /// The drawing window on the page.
    window: Bounds,
    pages: Vec<TestPage>,
    /// Where the next piece's box (label included) starts, from the window's corner.
    x: f64,
    y: f64,
    /// The tallest box in the current row.
    row_mm: f64,
}

/// Where the next whole piece goes on the current page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Spot {
    ThisRow,
    NextRow,
}

impl Shelves {
    fn new(window: Bounds) -> Self {
        Self {
            window,
            pages: Vec::new(),
            x: 0.0,
            y: 0.0,
            row_mm: 0.0,
        }
    }

    /// Where a box `length` × `height` would go on the current page, or `None` when the
    /// page has no room for it.
    fn spot(&self, (length, height): (f64, f64)) -> Option<Spot> {
        let room = (self.window.length_mm(), self.window.width_mm());
        if self.pages.is_empty() {
            return None;
        }
        let in_row = self.x == 0.0 || self.x + length <= room.0;
        if in_row && self.y + height <= room.1 {
            return Some(Spot::ThisRow);
        }
        let next_row = self.y + self.row_mm + GAP_MM;
        (length <= room.0 && next_row + height <= room.1).then_some(Spot::NextRow)
    }

    /// Puts copy `copy` of `piece`, item `index`, in the next free spot. `turned` is how
    /// it fits a page on its own; it turns the other way when only that keeps it on the
    /// current row, or else on the current page.
    fn add(
        &mut self,
        (index, piece, (copy, turned)): (usize, &SheetPiece, (u32, bool)),
    ) -> TestItem {
        let room = (self.window.length_mm(), self.window.width_mm());
        let size = |turned: bool| {
            let (length, width) = match turned {
                false => (piece.bounds.length_mm(), piece.bounds.width_mm()),
                true => (piece.bounds.width_mm(), piece.bounds.length_mm()),
            };
            (length, width + LABEL_MM)
        };
        let ways: Vec<bool> = [turned, !turned]
            .into_iter()
            .filter(|&way| {
                let (length, height) = size(way);
                length <= room.0 && height <= room.1
            })
            .collect();
        let pick = |wanted: Spot| {
            ways.iter()
                .copied()
                .find(|&way| self.spot(size(way)) == Some(wanted))
                .map(|way| (way, wanted))
        };
        let placed = pick(Spot::ThisRow).or_else(|| pick(Spot::NextRow));
        let turned = placed.map_or(turned, |(way, _)| way);
        match placed.map(|(_, spot)| spot) {
            Some(Spot::ThisRow) => {}
            Some(Spot::NextRow) => {
                self.y += self.row_mm + GAP_MM;
                (self.x, self.row_mm) = (0.0, 0.0);
            }
            None => {
                self.pages.push(TestPage::default());
                (self.x, self.y, self.row_mm) = (0.0, 0.0, 0.0);
            }
        }
        let item = TestItem::of((piece, (copy, turned)));
        let window = Bounds::from_size((item.length_mm, item.width_mm)).expanded(WHOLE_SLACK_MM);
        let corner = Point::new(
            self.window.min.x + self.x - WHOLE_SLACK_MM,
            self.window.min.y + self.y + LABEL_MM - WHOLE_SLACK_MM,
        );
        if let Some(page) = self.pages.last_mut() {
            page.views.push(View {
                item: index,
                window,
                at: corner,
            });
        }
        let (length, height) = size(turned);
        self.x += length + GAP_MM;
        self.row_mm = self.row_mm.max(height);
        item
    }

    fn into_pages(self) -> Vec<TestPage> {
        self.pages
    }
}
