//! Partition grid (CLAUDE.md §6.2): slotted dividers that cross like an egg crate, one
//! product per cell (the FEFCO 09xx interior fittings).
//!
//! - Strips along x ("long dividers") have slots cut up from the bottom. Strips along y
//!   ("short dividers") have slots cut down from the top. Every slot is the board
//!   thickness plus the slot tolerance wide, and half the height plus 0.5 mm deep, so the
//!   strips slide together with their tops and bottoms level.
//! - A cell's clear opening is the product's straightened footprint plus the clearance.
//!   In a box with a fixed inside size, the cells grow to fill it.
//! - With the outer frame on, strips also run round the outside, and every strip runs on
//!   past the corner crossings so the corner slots have board beyond them.

use super::hole::{FINGER_NOTCH_RADIUS_MM, HoleStyle, Lie, ProductHole};
use super::layout::FIT_SLACK_MM;
use super::{GenerateInsert, Insert, Mount, Stance, fixed_room, flat_piece};
use crate::design::{Design, PartitionGridSpec, Placement};
use crate::error::{DesignError, GenerateError};
use crate::folding::one_region;
use crate::geometry::{Area, Bounds, CURVE_TOLERANCE_MM, Path, Point, Polygon};
use crate::material::Material;
use crate::piece::Piece;

/// Extra slot depth past half the height, so crossing strips seat fully (CLAUDE.md §6.2).
const SLOT_EXTRA_DEPTH_MM: f64 = 0.5;
/// Smallest finger notch worth cutting into a strip's top edge.
const MIN_NOTCH_RADIUS_MM: f64 = 3.0;
/// How far slot cuts reach past the strip's edge, so no cut line runs along the edge.
const CUT_OVERSHOOT_MM: f64 = 1.0;

impl GenerateInsert for PartitionGridSpec {
    fn generate(&self, design: &Design) -> Result<Insert, GenerateError> {
        // Validation guarantees exactly one product.
        let Some(product) = design.products.first() else {
            return Err(DesignError::NoProducts.into());
        };
        let style = HoleStyle {
            clearance_mm: design.material.clearance_mm,
            finger_notches: None,
        };
        let hole = ProductHole::new(product, style)?;
        let plan = self.plan((design, &hole))?;
        let rules = StripRules::new((self, &design.material));

        let kinds = [
            StripKind {
                name: "long divider",
                length_mm: plan.x.footprint_mm,
                crossings: &plan.x.strips,
                slots_from: Edge::Bottom,
                stance: Stance::UprightAlongX,
                stands_at: &plan.y.strips,
            },
            StripKind {
                name: "short divider",
                length_mm: plan.y.footprint_mm,
                crossings: &plan.y.strips,
                slots_from: Edge::Top,
                stance: Stance::UprightAlongY,
                stands_at: &plan.x.strips,
            },
        ];
        let mut pieces = Vec::new();
        let mut mounts = Vec::new();
        for kind in kinds.iter().filter(|kind| !kind.stands_at.is_empty()) {
            let piece = rules.strip(kind)?;
            mounts.extend(kind.mounts((pieces.len(), self.divider_height_mm)));
            pieces.push(piece);
        }
        if pieces.is_empty() {
            return Err(GenerateError::EmptyGrid);
        }
        Ok(Insert {
            pieces,
            length_mm: plan.x.footprint_mm,
            width_mm: plan.y.footprint_mm,
            height_mm: self.divider_height_mm,
            placements: plan.placements((product.id.as_str(), product.quantity), &hole),
            mounts,
        })
    }
}

impl PartitionGridSpec {
    /// Cell sizes and strip positions: cells just big enough for the product, or grown to
    /// fill a box with a fixed inside size.
    fn plan(&self, (design, hole): (&Design, &ProductHole)) -> Result<GridPlan, GenerateError> {
        let frame = self.frame(&design.material);
        let cells = (self.columns, self.rows);
        // A cell's clear opening: the product's own footprint plus the clearance each side.
        let needed = |lie| -> (Lie, (f64, f64)) {
            let footprint = hole
                .product_footprint(lie)
                .unwrap_or(Bounds::from_size((0.0, 0.0)))
                .expanded(design.material.clearance_mm);
            (lie, (footprint.length_mm(), footprint.width_mm()))
        };
        let lengthwise = needed(Lie::Lengthwise);
        let Some(room) = fixed_room(design.outer_box.as_ref()) else {
            return Ok(GridPlan::new((cells, lengthwise), frame));
        };
        let fits = |(_, (length, width)): (Lie, (f64, f64))| {
            let cell = frame.cells_in((cells, room));
            length <= cell.0 + FIT_SLACK_MM && width <= cell.1 + FIT_SLACK_MM
        };
        let crosswise = needed(Lie::Crosswise);
        let Some((lie, _)) = [lengthwise, crosswise]
            .into_iter()
            .find(|&option| fits(option))
        else {
            let free = GridPlan::new((cells, lengthwise), frame);
            return Err(GenerateError::DoesNotFit {
                needed_length_mm: free.x.footprint_mm,
                needed_width_mm: free.y.footprint_mm,
                room_length_mm: room.0,
                room_width_mm: room.1,
            });
        };
        Ok(GridPlan::new(
            (cells, (lie, frame.cells_in((cells, room)))),
            frame,
        ))
    }

    fn frame(&self, material: &Material) -> Frame {
        let slot_width = material.thickness_mm + material.slot_tolerance_mm;
        Frame {
            thickness_mm: material.thickness_mm,
            // Board beyond the corner slot: half the slot, then a full web.
            end_tab_mm: self
                .outer_frame
                .then(|| slot_width / 2.0 + material.min_web_mm()),
        }
    }
}

/// How strips meet the outside of the grid.
#[derive(Debug, Clone, Copy)]
struct Frame {
    thickness_mm: f64,
    /// With a frame: how far each strip runs past the outermost crossing. `None` without
    /// a frame, where strips end at the box walls.
    end_tab_mm: Option<f64>,
}

impl Frame {
    /// What the frame adds to the footprint beyond the cells and the strips between them.
    fn extra_mm(self) -> f64 {
        self.end_tab_mm
            .map_or(0.0, |end_tab| self.thickness_mm + 2.0 * end_tab)
    }

    /// Cell openings that fill `room` exactly.
    fn cells_in(self, ((columns, rows), room): ((u32, u32), (f64, f64))) -> (f64, f64) {
        let cell = |(count, room): (u32, f64)| {
            let count = f64::from(count);
            (room - self.extra_mm() - (count - 1.0) * self.thickness_mm) / count
        };
        (cell((columns, room.0)), cell((rows, room.1)))
    }
}

/// One direction of the grid.
struct AxisPlan {
    /// Clear opening of a cell.
    cell_mm: f64,
    /// Where the first cell starts.
    first_cell_mm: f64,
    /// Centre lines of the strips standing across this direction.
    strips: Vec<f64>,
    footprint_mm: f64,
}

impl AxisPlan {
    fn new((cells, cell_mm): (u32, f64), frame: Frame) -> Self {
        let t = frame.thickness_mm;
        // With a frame, the first cell starts at the frame strip's inner face: the end tab
        // reaches the strip's centre line, then half a board.
        let first_cell_mm = frame.end_tab_mm.map_or(0.0, |end_tab| end_tab + t / 2.0);
        let pitch = cell_mm + t;
        // Strip j stands just before cell j; the frame adds the ones before the first cell
        // and after the last.
        let range = match frame.end_tab_mm {
            Some(_) => 0..=cells,
            None => 1..=cells.saturating_sub(1),
        };
        let strips = range
            .map(|j| first_cell_mm + f64::from(j) * pitch - t / 2.0)
            .collect();
        let cells = f64::from(cells);
        Self {
            cell_mm,
            first_cell_mm,
            strips,
            footprint_mm: cells * cell_mm + (cells - 1.0) * t + frame.extra_mm(),
        }
    }

    /// Centre of cell `index` along this direction.
    fn cell_center(&self, (index, thickness_mm): (u32, f64)) -> f64 {
        self.first_cell_mm + f64::from(index) * (self.cell_mm + thickness_mm) + self.cell_mm / 2.0
    }
}

/// The whole grid, in the insert's frame (footprint's top-left corner at 0, 0).
struct GridPlan {
    x: AxisPlan,
    y: AxisPlan,
    columns: u32,
    thickness_mm: f64,
    /// Which way products lie in their cells.
    lie: Lie,
}

impl GridPlan {
    fn new((cells, (lie, cell)): ((u32, u32), (Lie, (f64, f64))), frame: Frame) -> Self {
        let (columns, rows) = cells;
        Self {
            x: AxisPlan::new((columns, cell.0), frame),
            y: AxisPlan::new((rows, cell.1), frame),
            columns,
            thickness_mm: frame.thickness_mm,
            lie,
        }
    }

    /// One product copy centred in each cell, row by row from the top-left.
    fn placements(
        &self,
        (product_id, quantity): (&str, u32),
        hole: &ProductHole,
    ) -> Vec<Placement> {
        let footprint = hole
            .product_footprint(self.lie)
            .unwrap_or(Bounds::from_size((0.0, 0.0)));
        (0..quantity)
            .map(|copy| {
                let column = copy % self.columns.max(1);
                let row = copy / self.columns.max(1);
                let cell_center = Point::new(
                    self.x.cell_center((column, self.thickness_mm)),
                    self.y.cell_center((row, self.thickness_mm)),
                );
                Placement {
                    product_id: product_id.to_owned(),
                    center_mm: cell_center - footprint.center(),
                    rotation_deg: hole.rotation_deg(self.lie),
                }
            })
            .collect()
    }
}

/// The edge of a strip a slot opens from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Edge {
    Top,
    Bottom,
}

/// One kind of strip, cut as many times as there are places it stands.
struct StripKind<'a> {
    name: &'static str,
    length_mm: f64,
    /// Where the crossing strips meet it, measured along it from its start.
    crossings: &'a [f64],
    slots_from: Edge,
    stance: Stance,
    /// Where each copy stands, across its own direction.
    stands_at: &'a [f64],
}

impl StripKind<'_> {
    /// One mount per copy: x strips start at x = 0 and stand at a y; y strips the reverse.
    fn mounts(&self, (piece, height_mm): (usize, f64)) -> impl Iterator<Item = Mount> + '_ {
        self.stands_at.iter().map(move |&across| Mount {
            piece,
            stance: self.stance,
            at_mm: match self.stance {
                Stance::UprightAlongY => [across, 0.0, height_mm],
                _ => [0.0, across, height_mm],
            },
        })
    }
}

/// Sizes every strip shares.
struct StripRules {
    height_mm: f64,
    slot_width_mm: f64,
    slot_depth_mm: f64,
    /// Narrowest board to leave beside a slot or a strip end.
    web_mm: f64,
    finger_notches: bool,
}

impl StripRules {
    fn new((grid, material): (&PartitionGridSpec, &Material)) -> Self {
        Self {
            height_mm: grid.divider_height_mm,
            slot_width_mm: material.thickness_mm + material.slot_tolerance_mm,
            slot_depth_mm: grid.divider_height_mm / 2.0 + SLOT_EXTRA_DEPTH_MM,
            web_mm: material.min_web_mm(),
            finger_notches: grid.finger_notches,
        }
    }

    /// The flat strip: a rectangle (top edge at y = 0) with its slots and finger notches.
    fn strip(&self, kind: &StripKind) -> Result<Piece, GenerateError> {
        let board =
            Area::from_polygon(&Bounds::from_size((kind.length_mm, self.height_mm)).to_polygon())?;
        let mut cuts: Vec<Polygon> = kind
            .crossings
            .iter()
            .map(|&along| self.slot((along, kind.slots_from)))
            .collect();
        if self.finger_notches {
            cuts.extend(self.notches(kind));
        }
        let cut = board.difference(&Area::from_polygons(&cuts)?)?;
        let region = one_region((kind.name, cut))?;
        let mut piece = flat_piece((kind.name, region)).ok_or(GenerateError::FallsApart {
            piece: kind.name,
            parts: 0,
        })?;
        piece.quantity = u32::try_from(kind.stands_at.len()).unwrap_or(u32::MAX);
        Ok(piece)
    }

    /// A slot centred `along` the strip, opening from `edge`, reaching a little past it.
    fn slot(&self, (along, edge): (f64, Edge)) -> Polygon {
        let (top, bottom) = match edge {
            Edge::Top => (-CUT_OVERSHOOT_MM, self.slot_depth_mm),
            Edge::Bottom => (
                self.height_mm - self.slot_depth_mm,
                self.height_mm + CUT_OVERSHOOT_MM,
            ),
        };
        let half = self.slot_width_mm / 2.0;
        Bounds {
            min: Point::new(along - half, top),
            max: Point::new(along + half, bottom),
        }
        .to_polygon()
    }

    /// Half-round notches in the top edge, one in the middle of each stretch between
    /// crossings (and strip ends) that has room for one.
    fn notches(&self, kind: &StripKind) -> Vec<Polygon> {
        let beside_slot = self.slot_width_mm / 2.0 + self.web_mm;
        let mut stops: Vec<(f64, f64)> = vec![(0.0, self.web_mm)];
        stops.extend(kind.crossings.iter().map(|&along| (along, beside_slot)));
        stops.push((kind.length_mm, self.web_mm));
        stops.sort_by(|a, b| a.0.total_cmp(&b.0));
        stops
            .windows(2)
            .filter_map(|pair| {
                let start = pair[0].0 + pair[0].1;
                let end = pair[1].0 - pair[1].1;
                let radius = FINGER_NOTCH_RADIUS_MM
                    .min(self.height_mm / 2.0)
                    .min((end - start) / 2.0);
                (radius >= MIN_NOTCH_RADIUS_MM).then(|| {
                    let center = Point::new((start + end) / 2.0, 0.0);
                    Polygon(Path::circle(center, radius).flatten(CURVE_TOLERANCE_MM))
                })
            })
            .collect()
    }
}
