//! Building cut pieces: a cut that must leave one region, pieces that fold (panels joined
//! by hinges, cut round one outline), and the lock tabs that hold folded board without
//! glue. Shared by the inserts and the outer boxes.

use crate::error::GenerateError;
use crate::geometry::{Area, Bounds, Line, Path, Point, Polygon, Region, Segment, Transform};
use crate::piece::{FoldDirection, Hinge, Panel, Piece};

/// Length of a lock tab along its edge, when there's room.
const TAB_LENGTH_MM: f64 = 15.0;
/// Shortest lock tab worth cutting.
const MIN_TAB_LENGTH_MM: f64 = 5.0;
/// About one lock tab per this much edge, and at least two when they fit.
const TAB_PITCH_MM: f64 = 60.0;

/// The one region a cut must leave, or [`GenerateError::FallsApart`] naming `piece`.
pub(crate) fn one_region((piece, area): (&'static str, Area)) -> Result<Region, GenerateError> {
    let falls_apart = |parts| GenerateError::FallsApart { piece, parts };
    let [region] =
        <[_; 1]>::try_from(area.into_regions()).map_err(|regions| falls_apart(regions.len()))?;
    Ok(region)
}

/// The rectangle between two x values and two y values, each pair in either order.
pub(crate) fn rect(((x0, x1), (y0, y1)): ((f64, f64), (f64, f64))) -> Polygon {
    Bounds {
        min: Point::new(x0.min(x1), y0.min(y1)),
        max: Point::new(x0.max(x1), y0.max(y1)),
    }
    .to_polygon()
}

/// The straight line along y = `y`, from x0 to x1.
pub(crate) fn along_x((y, (x0, x1)): (f64, (f64, f64))) -> Line {
    Line {
        from: Point::new(x0, y),
        to: Point::new(x1, y),
    }
}

/// The straight line along x = `x`, from y0 to y1.
pub(crate) fn along_y((x, (y0, y1)): (f64, (f64, f64))) -> Line {
    Line {
        from: Point::new(x, y0),
        to: Point::new(x, y1),
    }
}

/// Where a panel joins its parent, and how it folds.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Joint {
    /// Index of the parent panel, as returned by [`FoldingPiece::fold`] (0 for the root).
    pub parent: usize,
    /// The crease, on the edge both panels share.
    pub axis: Line,
    /// Angle when fully folded, 0–180.
    pub deg: f64,
    pub direction: FoldDirection,
    /// How far past the crease the panel turns (see [`Hinge::offset_mm`]).
    pub offset_mm: f64,
    /// Fold order for the animation: lower stages fold first.
    pub stage: u32,
}

/// A piece under construction, drawn in whatever frame suits the generator.
pub(crate) struct FoldingPiece {
    name: &'static str,
    panels: Vec<Area>,
    hinges: Vec<Hinge>,
}

/// A finished piece, and where its drawing frame was.
pub(crate) struct BuiltPiece {
    pub piece: Piece,
    /// Where the piece's (0, 0) sat in the generator's frame: its top-left corner.
    pub flat_min: Point,
}

impl FoldingPiece {
    /// Starts a piece from its root panel (panel 0).
    pub(crate) fn new((name, root): (&'static str, Area)) -> Self {
        Self {
            name,
            panels: vec![root],
            hinges: Vec::new(),
        }
    }

    /// Adds `child`, folding from its parent along the joint, and returns its index.
    pub(crate) fn fold(&mut self, (child, joint): (Area, Joint)) -> usize {
        let index = self.panels.len();
        self.panels.push(child);
        self.hinges.push(Hinge {
            parent: joint.parent,
            child: index,
            axis: joint.axis,
            fold_deg: joint.deg,
            direction: joint.direction,
            offset_mm: joint.offset_mm,
            stage: joint.stage,
        });
        index
    }

    /// Cuts the piece: one outline round all the panels, the holes inside it, and a
    /// crease along every hinge. Every panel, and the whole piece, must be one region.
    /// The drawing moves so the piece's top-left corner lands on (0, 0).
    pub(crate) fn build(self) -> Result<BuiltPiece, GenerateError> {
        let name = self.name;
        let whole = one_region((name, Area::union_all(&self.panels)?))?;
        let flat_min = whole
            .outer
            .bounds()
            .map_or(Point::ORIGIN, |bounds| bounds.min);
        let to_piece = Transform::shift(Point::ORIGIN - flat_min);
        let panels = self
            .panels
            .into_iter()
            .map(|area| Ok(placed((one_region((name, area))?, to_piece))))
            .collect::<Result<Vec<_>, GenerateError>>()?;
        let hinges: Vec<Hinge> = self
            .hinges
            .into_iter()
            .map(|hinge| Hinge {
                axis: Line {
                    from: to_piece.apply(hinge.axis.from),
                    to: to_piece.apply(hinge.axis.to),
                },
                ..hinge
            })
            .collect();
        let whole = placed((whole, to_piece));
        let outline = whole.outline.to_path().ok_or(GenerateError::FallsApart {
            piece: name,
            parts: 0,
        })?;
        let piece = Piece {
            name: name.to_owned(),
            quantity: 1,
            outline,
            holes: whole.holes.iter().filter_map(Polygon::to_path).collect(),
            creases: hinges.iter().map(|hinge| crease(hinge.axis)).collect(),
            perforations: Vec::new(),
            panels,
            hinges,
        };
        Ok(BuiltPiece { piece, flat_min })
    }
}

/// A region as a panel, moved by `transform`.
fn placed((region, transform): (Region, Transform)) -> Panel {
    Panel {
        outline: region.outer.transformed(transform),
        holes: region
            .holes
            .iter()
            .map(|hole| hole.transformed(transform))
            .collect(),
    }
}

/// The crease drawn along a hinge's axis.
fn crease(axis: Line) -> Path {
    Path {
        start: axis.from,
        segments: vec![Segment::Line { to: axis.to }],
        closed: false,
    }
}

/// Lock tabs on the free edge of a folded panel, and the slots they push through.
#[derive(Debug, Clone, Copy)]
pub(crate) struct LockTabs {
    /// Board to leave at each end of the edge and between neighbouring slots.
    pub web_mm: f64,
    /// Added to a tab's length to get its slot's length.
    pub tolerance_mm: f64,
}

impl LockTabs {
    /// Where each tab runs along an edge of `length_mm`, as (start, end) from the edge's
    /// start, or `None` when not even one shortest tab fits.
    ///
    /// The tabs share the edge between a web at each end equally, each centred in its
    /// share, with at least a web between neighbouring slots. Two or more tabs if they
    /// fit, otherwise one.
    pub(crate) fn spans(self, length_mm: f64) -> Option<Vec<(f64, f64)>> {
        let band = length_mm - 2.0 * self.web_mm;
        let tab_for = |count: u32| {
            (band / f64::from(count) - self.web_mm - self.tolerance_mm).min(TAB_LENGTH_MM)
        };
        // `as` saturates, and edges are at most a few metres long.
        let many = ((band / TAB_PITCH_MM) as u32).max(2);
        let (count, tab) = [many, 1]
            .into_iter()
            .map(|count| (count, tab_for(count)))
            .find(|&(_, tab)| tab >= MIN_TAB_LENGTH_MM)?;
        let share = band / f64::from(count);
        Some(
            (0..count)
                .map(|index| {
                    let center = self.web_mm + share * (f64::from(index) + 0.5);
                    (center - tab / 2.0, center + tab / 2.0)
                })
                .collect(),
        )
    }

    /// Shortest edge that takes one tab: a web at each end, and a web plus the
    /// tolerance round the slot.
    pub(crate) fn min_edge_mm(self) -> f64 {
        3.0 * self.web_mm + self.tolerance_mm + MIN_TAB_LENGTH_MM
    }
}
