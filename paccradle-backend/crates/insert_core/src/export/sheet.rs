//! The layout every production file draws (M1-B): each piece once, laid out in rows on
//! one drawing, its lines sorted into cut, crease and perforation.
//!
//! - Insert pieces come first, then the box pieces, each in the order generated.
//! - Each piece is drawn once; [`SheetPiece::quantity`] says how many to cut. A die maker
//!   builds one die per piece, and cutter apps duplicate a shape in one click.
//! - Box pieces are mirrored (x → −x): the engine draws a box from the inside, so this
//!   turns its print side up, as printers expect (M1-A6). Inserts stay as drawn.
//! - Lines keep the engine's exact geometry, arcs included.
//! - Hinges come along as [`Fold`]s, numbered in folding order, for the test print's fold
//!   steps (B4).

use crate::check::Part;
use crate::geometry::{Bounds, CURVE_TOLERANCE_MM, Line, Path, Point, Segment};
use crate::packaging::Packaging;
use crate::piece::{FoldDirection, Piece};

/// Space round the drawing and between pieces, in mm.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spacing {
    pub margin_mm: f64,
    pub gap_mm: f64,
}

impl Spacing {
    /// Cut files (SVG, DXF): enough to keep pieces apart.
    pub const CUT_FILE: Spacing = Spacing {
        margin_mm: 10.0,
        gap_mm: 10.0,
    };

    /// Drawings people read (PDF): room round each piece for its label above and its
    /// dimensions above and to the left.
    pub const DRAWING: Spacing = Spacing {
        margin_mm: 25.0,
        gap_mm: 25.0,
    };
}

/// Rows of pieces wrap past this length: the long side of a B1 press sheet
/// (1000 × 700 mm). A piece longer than that gets a row of its own.
const ROW_MM: f64 = 1000.0;

/// What a line does to the board. Each kind is one layer in a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LineKind {
    Cut,
    Crease,
    /// A cut line broken into short cuts, to tear along.
    Perforation,
}

impl LineKind {
    /// Every kind, in drawing order.
    pub const ALL: [LineKind; 3] = [LineKind::Cut, LineKind::Crease, LineKind::Perforation];

    /// The layer's name in files: what die makers and cutter apps look for.
    pub const fn layer_name(self) -> &'static str {
        match self {
            LineKind::Cut => "CUT",
            LineKind::Crease => "CREASE",
            LineKind::Perforation => "PERFORATION",
        }
    }

    /// The layer's colour as `#rrggbb`: the common laser convention of red to cut and blue
    /// to score, and green for perforations.
    pub const fn color(self) -> &'static str {
        match self {
            LineKind::Cut => "#ff0000",
            LineKind::Crease => "#0000ff",
            LineKind::Perforation => "#00a000",
        }
    }
}

/// The drawing every production file shows.
#[derive(Debug, Clone, PartialEq)]
pub struct Sheet {
    /// The drawing's size along x, margins included.
    pub length_mm: f64,
    /// The drawing's size along y, margins included.
    pub width_mm: f64,
    /// The space it was laid out with.
    pub spacing: Spacing,
    pub pieces: Vec<SheetPiece>,
}

/// One piece placed on the sheet, in sheet coordinates (mm, x right, y down).
#[derive(Debug, Clone, PartialEq)]
pub struct SheetPiece {
    /// The insert or the box.
    pub part: Part,
    /// Index into that part's pieces.
    pub index: usize,
    pub name: String,
    /// How many to cut.
    pub quantity: u32,
    /// True for box pieces, drawn mirrored so their print side faces up.
    pub mirrored: bool,
    /// Where the piece's outline sits on the sheet.
    pub bounds: Bounds,
    /// The outline first, then the holes.
    pub cuts: Vec<Path>,
    pub creases: Vec<Path>,
    pub perforations: Vec<Path>,
    /// The piece's hinges, one per [`Piece::hinges`] entry.
    pub folds: Vec<Fold>,
}

/// A crease that folds, on the sheet.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fold {
    /// The crease, in sheet coordinates.
    pub axis: Line,
    /// When it folds within its piece: 1 first. Hinges of one stage share a step.
    pub step: u32,
    /// Which way it folds, seen on the sheet. A mirrored box piece shows its print side,
    /// so its valley folds (drawn from the inside) show as mountain folds.
    pub direction: FoldDirection,
    /// Angle when fully folded, 0–180.
    pub fold_deg: f64,
}

impl SheetPiece {
    /// What the piece is: "Insert: card", "Box: mailer". Plain ASCII, for every file
    /// format.
    pub fn label(&self) -> String {
        let part = match self.part {
            Part::Insert => "Insert",
            Part::OuterBox => "Box",
        };
        format!("{part}: {}", self.name)
    }

    /// What the piece is and how many to cut: "Insert: card x 1", "Box: mailer x 1
    /// (mirrored: print side up)".
    pub fn title(&self) -> String {
        let mirrored = match self.mirrored {
            true => " (mirrored: print side up)",
            false => "",
        };
        format!("{} x {}{mirrored}", self.label(), self.quantity)
    }

    /// The piece's lines of one kind.
    pub fn lines(&self, kind: LineKind) -> &[Path] {
        match kind {
            LineKind::Cut => &self.cuts,
            LineKind::Crease => &self.creases,
            LineKind::Perforation => &self.perforations,
        }
    }
}

impl Sheet {
    /// Lays out every piece of `packaging` with `spacing`: insert pieces, then box
    /// pieces mirrored.
    pub fn of((packaging, spacing): (&Packaging, Spacing)) -> Sheet {
        let insert = packaging
            .insert
            .pieces
            .iter()
            .enumerate()
            .map(|(index, piece)| (Part::Insert, index, piece));
        let outer_box = packaging.outer_box.iter().flat_map(|outer_box| {
            outer_box
                .pieces
                .iter()
                .enumerate()
                .map(|(index, piece)| (Part::OuterBox, index, piece))
        });
        let mut rows = Rows::new(spacing);
        let pieces = insert
            .chain(outer_box)
            .filter_map(|entry| place((entry, &mut rows)))
            .collect();
        let (length_mm, width_mm) = rows.size();
        Sheet {
            length_mm,
            width_mm,
            spacing,
            pieces,
        }
    }
}

/// Places one piece in the next free spot; `None` for a piece with no outline.
fn place(((part, index, piece), rows): ((Part, usize, &Piece), &mut Rows)) -> Option<SheetPiece> {
    let own = Bounds::of(piece.outline.flatten(CURVE_TOLERANCE_MM))?;
    let corner = rows.next((own.length_mm(), own.width_mm()));
    let mirrored = part == Part::OuterBox;
    let lay = Lay {
        own,
        corner,
        mirrored,
    };
    let lay_all = |paths: &[Path]| paths.iter().map(|path| lay.path(path)).collect();
    Some(SheetPiece {
        part,
        index,
        name: piece.name.clone(),
        quantity: piece.quantity,
        mirrored,
        bounds: Bounds {
            min: corner,
            max: Point::new(corner.x + own.length_mm(), corner.y + own.width_mm()),
        },
        cuts: std::iter::once(lay.path(&piece.outline))
            .chain(piece.holes.iter().map(|hole| lay.path(hole)))
            .collect(),
        creases: lay_all(&piece.creases),
        perforations: lay_all(&piece.perforations),
        folds: folds((piece, &lay)),
    })
}

/// The piece's hinges on the sheet, numbered in folding order by their stages.
fn folds((piece, lay): (&Piece, &Lay)) -> Vec<Fold> {
    let mut stages: Vec<u32> = piece.hinges.iter().map(|hinge| hinge.stage).collect();
    stages.sort_unstable();
    stages.dedup();
    piece
        .hinges
        .iter()
        .map(|hinge| Fold {
            axis: Line {
                from: lay.point(hinge.axis.from),
                to: lay.point(hinge.axis.to),
            },
            step: stages.partition_point(|&stage| stage < hinge.stage) as u32 + 1,
            direction: match lay.mirrored {
                true => hinge.direction.flipped(),
                false => hinge.direction,
            },
            fold_deg: hinge.fold_deg,
        })
        .collect()
}

/// How a piece's own drawing maps onto the sheet: its box's top-left corner to
/// `corner`, mirrored left to right when `mirrored`.
struct Lay {
    own: Bounds,
    corner: Point,
    mirrored: bool,
}

impl Lay {
    fn point(&self, point: Point) -> Point {
        let x = match self.mirrored {
            true => self.own.max.x - point.x,
            false => point.x - self.own.min.x,
        };
        Point::new(self.corner.x + x, self.corner.y + point.y - self.own.min.y)
    }

    /// The path on the sheet. Mirroring turns arcs the other way round.
    fn path(&self, path: &Path) -> Path {
        let segments = path
            .segments
            .iter()
            .map(|segment| match *segment {
                Segment::Line { to } => Segment::Line { to: self.point(to) },
                Segment::Arc { center, sweep_deg } => Segment::Arc {
                    center: self.point(center),
                    sweep_deg: match self.mirrored {
                        true => -sweep_deg,
                        false => sweep_deg,
                    },
                },
            })
            .collect();
        Path {
            start: self.point(path.start),
            segments,
            closed: path.closed,
        }
    }
}

/// Fills rows left to right, top to bottom.
struct Rows {
    spacing: Spacing,
    /// Where the next piece goes in the current row.
    x: f64,
    /// The current row's top.
    y: f64,
    /// The tallest piece in the current row.
    row_mm: f64,
    /// The furthest any piece reaches along x.
    reach_mm: f64,
}

impl Rows {
    fn new(spacing: Spacing) -> Self {
        let margin = spacing.margin_mm;
        Self {
            spacing,
            x: margin,
            y: margin,
            row_mm: 0.0,
            reach_mm: margin,
        }
    }

    /// The top-left corner for a piece of this size, starting a new row when it would
    /// run past [`ROW_MM`].
    fn next(&mut self, (length_mm, width_mm): (f64, f64)) -> Point {
        let Spacing { margin_mm, gap_mm } = self.spacing;
        let row_started = self.x > margin_mm;
        if row_started && self.x + length_mm > margin_mm + ROW_MM {
            self.x = margin_mm;
            self.y += self.row_mm + gap_mm;
            self.row_mm = 0.0;
        }
        let corner = Point::new(self.x, self.y);
        self.x += length_mm + gap_mm;
        self.row_mm = self.row_mm.max(width_mm);
        self.reach_mm = self.reach_mm.max(corner.x + length_mm);
        corner
    }

    /// The whole drawing's size, margins included.
    fn size(&self) -> (f64, f64) {
        let margin = self.spacing.margin_mm;
        (self.reach_mm + margin, self.y + self.row_mm + margin)
    }
}
