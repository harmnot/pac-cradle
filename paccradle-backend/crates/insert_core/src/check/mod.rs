//! Manufacturability checks (CLAUDE.md §6.5): what a printer or die maker would flag in
//! the generated pieces.
//!
//! Each [`Warning`] names its [`Rule`], with the numbers behind it, and the [`Spot`]: the
//! piece, the box round the feature on the piece's flat drawing, and the product copies
//! involved. Warnings never stop a design. The editor lists them (M1-E8) and the user
//! decides.
//!
//! The rules, by module:
//! - `holes`: holes closer than a web (3 mm or 3× the board) to each other or to the
//!   piece's edge, and product holes closer than 2× the board to a crease. The hole
//!   rules look at every product copy's own hole, so products that overlap (one merged
//!   hole) or that run off the card (no hole at all) are caught too. Lock slots sit
//!   beside their creases on purpose, so the crease rule skips them.
//! - `board`: a board thicker or thinner than usual for its kind, and slots too tight or
//!   too loose for the board.
//! - `small_features`: board, or a cut-out, narrower than 1 mm.
//! - `cut_lines`: cut lines that don't close, lines drawn twice, and creases that run
//!   into a cut line.
//! - `panels`: a panel more than 60 % holes, and a piece bigger than the chosen sheet.
//! - `cradle`: a product no taller than the deck, or standing over a leg.
//!
//! Run them with [`Packaging::warnings`], apart from generating: they cost more than a
//! preview frame.

mod board;
mod cradle;
mod cut_lines;
mod holes;
mod panels;
mod small_features;

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::design::Design;
use crate::error::GeometryError;
use crate::geometry::{Bounds, CURVE_TOLERANCE_MM, Line, Path, Point, Polygon};
use crate::insert::{Mount, Stance};
use crate::material::{Board, Material, MmRange};
use crate::packaging::Packaging;
use crate::piece::Piece;

/// Narrowest board or cut-out a cutter handles cleanly (CLAUDE.md §6.5).
pub const MIN_FEATURE_MM: f64 = 1.0;
/// A product hole keeps this many board thicknesses from a crease (CLAUDE.md §6.5).
pub const CREASE_CLEARANCE_BOARDS: f64 = 2.0;
/// Largest share of a panel that holes may take before it gets weak (CLAUDE.md §6.5,
/// an estimate).
pub const MAX_CUT_OUT_SHARE: f64 = 0.6;
/// Slot tolerance (slot width less the board) that still fits: below it the board won't
/// slide in, above it the board wobbles. An estimate round the §6.4 starting values
/// (0.3 mm for solid board, 0.5 mm for corrugated).
pub const SLOT_PLAY_MM: MmRange = MmRange { min: 0.1, max: 1.0 };
/// Distances within this of a limit pass: the polygon library snaps to 0.1 µm.
const SLACK_MM: f64 = 1e-3;

/// What to check against, beyond the design.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct CheckOptions {
    /// The sheet or cutting mat the pieces are cut from; `None` skips the size rule.
    pub sheet: Option<SheetSize>,
}

/// A sheet or cutting mat, in mm. A piece fits either way round.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SheetSize {
    pub length_mm: f64,
    pub width_mm: f64,
}

/// One thing to fix before cutting, and where it is.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Warning {
    pub rule: Rule,
    pub at: Spot,
}

/// Which rule a warning comes from, with the numbers behind it (mm unless named).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Rule {
    /// Two holes with less than a web of board between them; 0 when they overlap.
    HolesTooClose { gap_mm: f64, min_mm: f64 },
    /// A hole less than a web from the piece's edge; 0 when it reaches or crosses it.
    HoleNearEdge { gap_mm: f64, min_mm: f64 },
    /// A product hole less than 2× the board from a crease; 0 when it reaches it.
    HoleNearCrease { gap_mm: f64, min_mm: f64 },
    /// Slots too close to the board's thickness for it to slide in.
    SlotTooTight { slot_mm: f64, board_mm: f64 },
    /// Slots so much wider than the board that it wobbles.
    SlotTooLoose { slot_mm: f64, board_mm: f64 },
    /// Board, or a cut-out, narrower than [`MIN_FEATURE_MM`]; `size_mm` is its rough width.
    TooSmall {
        feature: SmallFeature,
        size_mm: f64,
        min_mm: f64,
    },
    /// A crease that crosses or ends on a cut line, away from its own ends.
    CreaseCrossesCut,
    /// A cut line that doesn't join back to its start.
    OpenCut,
    /// Two lines on top of each other for `length_mm`: die makers cut each line once.
    DuplicateLine { length_mm: f64 },
    /// A piece bigger than the chosen sheet either way round.
    TooBigForSheet {
        length_mm: f64,
        width_mm: f64,
        sheet: SheetSize,
    },
    /// A panel whose holes take more than `max_share` of it (shares are 0–1).
    WeakPanel { cut_out_share: f64, max_share: f64 },
    /// A cradle product no taller than the deck: it sinks below the deck's top.
    ProductBelowDeck {
        product_id: String,
        height_mm: f64,
        deck_height_mm: f64,
    },
    /// A cradle product standing over a leg, which runs under the deck's long edge.
    ProductOverLeg { product_id: String },
    /// A board thickness outside the usual range for its kind (CLAUDE.md §6.4).
    UnusualThickness {
        board: Board,
        thickness_mm: f64,
        typical: MmRange,
    },
}

/// What [`Rule::TooSmall`] found too small.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SmallFeature {
    /// A strip of board: it tears.
    Board,
    /// A slot, slit or gap in the cut lines: two cuts that close can't be cut cleanly.
    CutOut,
}

/// Where a warning points.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Spot {
    pub part: Part,
    /// Index into that part's pieces; `None` for the whole part (a board setting).
    pub piece: Option<usize>,
    /// The box round the feature on the piece's flat drawing, in the piece's own
    /// coordinates (as `Piece::outline`); `None` for the whole piece.
    pub area_mm: Option<Bounds>,
    /// The product copies involved, as indices into `Insert::placements`.
    pub placements: Vec<usize>,
}

/// The insert or the outer box.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Part {
    Insert,
    OuterBox,
}

impl Spot {
    /// A whole piece.
    fn piece((part, piece): (Part, usize)) -> Self {
        Self {
            part,
            piece: Some(piece),
            area_mm: None,
            placements: Vec::new(),
        }
    }

    /// The same piece, pointing at `area_mm`.
    fn area(self, area_mm: Option<Bounds>) -> Self {
        Self { area_mm, ..self }
    }

    /// The same spot, naming the product copies involved.
    fn placements(self, placements: Vec<usize>) -> Self {
        Self { placements, ..self }
    }
}

impl Packaging {
    /// The manufacturability warnings for this packaging, generated from `design`
    /// (CLAUDE.md §6.5). An empty list means nothing to flag.
    pub fn warnings(
        &self,
        (design, options): (&Design, &CheckOptions),
    ) -> Result<Vec<Warning>, GeometryError> {
        let context = Context {
            design,
            packaging: self,
            options,
        };
        let mut warnings = Vec::new();
        for check in CHECKS {
            warnings.extend(check.run(&context)?);
        }
        Ok(warnings)
    }
}

/// One group of rules.
trait Check {
    fn run(&self, context: &Context) -> Result<Vec<Warning>, GeometryError>;
}

const CHECKS: [&dyn Check; 6] = [
    &holes::HoleSpacing,
    &board::BoardSettings,
    &small_features::SmallFeatures,
    &cut_lines::CutLines,
    &panels::Panels,
    &cradle::CradleProducts,
];

/// What every check looks at.
struct Context<'a> {
    design: &'a Design,
    packaging: &'a Packaging,
    options: &'a CheckOptions,
}

/// One cut piece, and the board it's cut from.
#[derive(Clone, Copy)]
struct PieceRef<'a> {
    part: Part,
    index: usize,
    piece: &'a Piece,
    material: &'a Material,
}

impl PieceRef<'_> {
    fn spot(&self) -> Spot {
        Spot::piece((self.part, self.index))
    }
}

impl Context<'_> {
    /// Every piece of the insert, then of the box.
    fn pieces(&self) -> Vec<PieceRef<'_>> {
        let insert = self
            .packaging
            .insert
            .pieces
            .iter()
            .enumerate()
            .map(|(index, piece)| PieceRef {
                part: Part::Insert,
                index,
                piece,
                material: &self.design.material,
            });
        let outer_box = self
            .packaging
            .outer_box
            .as_ref()
            .zip(self.design.outer_box.as_ref())
            .into_iter()
            .flat_map(|(outer_box, spec)| {
                outer_box
                    .pieces
                    .iter()
                    .enumerate()
                    .map(|(index, piece)| PieceRef {
                        part: Part::OuterBox,
                        index,
                        piece,
                        material: &spec.material,
                    })
            });
        insert.chain(outer_box).collect()
    }

    /// The insert piece that lies flat, holding the products in their holes (the card,
    /// the cradle), and the shift from the insert's frame to that piece's drawing.
    fn flat_insert_piece(&self) -> Option<(usize, Point)> {
        self.packaging
            .insert
            .mounts
            .iter()
            .find(|mount| mount.stance == Stance::Flat)
            .map(|mount: &Mount| (mount.piece, Point::new(-mount.at_mm[0], -mount.at_mm[1])))
    }
}

/// A path as a polygon (closed paths) or a chain of points (open ones), arcs as chords.
fn points(path: &Path) -> Polygon {
    Polygon(path.flatten(CURVE_TOLERANCE_MM))
}

/// The straight lines between a path's points, closing the ring for a closed path.
fn lines(path: &Path) -> Vec<Line> {
    let ring = points(path);
    match path.closed {
        true => ring.edges().collect(),
        false => ring
            .points()
            .windows(2)
            .map(|pair| Line {
                from: pair[0],
                to: pair[1],
            })
            .collect(),
    }
}

/// The box round both, when there are both.
fn union(a: Option<Bounds>, b: Option<Bounds>) -> Option<Bounds> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.union(b)),
        (a, b) => a.or(b),
    }
}

impl fmt::Display for Rule {
    /// The warning in English, for logs and as the editor's fallback text.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HolesTooClose { gap_mm, min_mm } if *gap_mm <= 0.0 => write!(
                f,
                "holes overlap; leave at least {min_mm:.1} mm of board between holes"
            ),
            Self::HolesTooClose { gap_mm, min_mm } => write!(
                f,
                "holes are {gap_mm:.1} mm apart; leave at least {min_mm:.1} mm of board between holes"
            ),
            Self::HoleNearEdge { gap_mm, min_mm } if *gap_mm <= 0.0 => write!(
                f,
                "a hole reaches the edge; keep holes at least {min_mm:.1} mm in"
            ),
            Self::HoleNearEdge { gap_mm, min_mm } => write!(
                f,
                "a hole is {gap_mm:.1} mm from the edge; keep holes at least {min_mm:.1} mm in"
            ),
            Self::HoleNearCrease { gap_mm, min_mm } => write!(
                f,
                "a hole is {gap_mm:.1} mm from a crease; keep at least {min_mm:.1} mm"
            ),
            Self::SlotTooTight { slot_mm, board_mm } => write!(
                f,
                "slots are {slot_mm:.2} mm wide for {board_mm:.2} mm board: too tight to slide together"
            ),
            Self::SlotTooLoose { slot_mm, board_mm } => write!(
                f,
                "slots are {slot_mm:.2} mm wide for {board_mm:.2} mm board: too loose to hold"
            ),
            Self::TooSmall {
                feature: SmallFeature::Board,
                size_mm,
                min_mm,
            } => write!(
                f,
                "a strip of board only about {size_mm:.2} mm wide; under {min_mm} mm it tears"
            ),
            Self::TooSmall {
                feature: SmallFeature::CutOut,
                size_mm,
                min_mm,
            } => write!(
                f,
                "a cut-out only about {size_mm:.2} mm wide; under {min_mm} mm it can't be cut cleanly"
            ),
            Self::CreaseCrossesCut => write!(f, "a crease runs into a cut line"),
            Self::OpenCut => write!(f, "a cut line doesn't close"),
            Self::DuplicateLine { length_mm } => write!(
                f,
                "two lines lie on top of each other for {length_mm:.1} mm; cut each line once"
            ),
            Self::TooBigForSheet {
                length_mm,
                width_mm,
                sheet,
            } => write!(
                f,
                "the piece is {length_mm:.0} × {width_mm:.0} mm, bigger than the {:.0} × {:.0} mm sheet",
                sheet.length_mm, sheet.width_mm
            ),
            Self::WeakPanel {
                cut_out_share,
                max_share,
            } => write!(
                f,
                "holes take {:.0} % of the panel; above {:.0} % it may be too weak",
                cut_out_share * 100.0,
                max_share * 100.0
            ),
            Self::ProductBelowDeck {
                product_id,
                height_mm,
                deck_height_mm,
            } => write!(
                f,
                "`{product_id}` is {height_mm:.1} mm tall, no taller than the {deck_height_mm:.1} mm deck: it sinks below the deck"
            ),
            Self::ProductOverLeg { product_id } => {
                write!(f, "`{product_id}` stands over a leg under the deck")
            }
            Self::UnusualThickness {
                board,
                thickness_mm,
                typical,
            } => write!(
                f,
                "{} is usually {}–{} mm thick, not {thickness_mm} mm",
                board_name(*board),
                typical.min,
                typical.max
            ),
        }
    }
}

/// The board's name in English.
fn board_name(board: Board) -> &'static str {
    match board {
        Board::Paperboard => "paperboard",
        Board::Chipboard => "chipboard",
        Board::EFlute => "E-flute",
        Board::BFlute => "B-flute",
        Board::CFlute => "C-flute",
    }
}
