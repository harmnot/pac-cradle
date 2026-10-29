//! The 2D fit check (CLAUDE.md §7.3): how much room each product copy has round it in the
//! insert, seen from above, as a colour for the editor.
//!
//! - Green: at least the clearance recommended for the insert's board (§6.4), within the
//!   0.01 mm accuracy bar.
//! - Yellow: it fits, but tighter than that.
//! - Red: it runs into something (a [`Clash`]), or comes within 0.01 mm of it.
//!
//! The generator cuts every hole at the design's own clearance, so a product only turns
//! yellow when the design asks for less clearance than the board's recommended value, and
//! red when it clashes: two products overlapping in one merged hole, a product hanging off
//! the card, a product standing over a cradle leg.
//!
//! What surrounds a product, seen from above: the board of every piece lying flat (the
//! card, the cradle's deck), with the product in one of its holes; every piece standing
//! upright (grid strips), a board thick; the cradle's legs, under the deck's long edges;
//! and the insert's own edge.

use serde::{Deserialize, Serialize};

use crate::design::{Design, InsertSpec};
use crate::error::GeometryError;
use crate::geometry::{Bounds, CURVE_TOLERANCE_MM, Point, Polygon, Ring, Transform};
use crate::insert::{Insert, PlacedProduct, Stance, placed_products};
use crate::material::Material;
use crate::packaging::Packaging;

/// Gaps within this of a limit count as reaching it: the 0.01 mm accuracy bar (M1-A2).
const FIT_SLACK_MM: f64 = 0.01;

/// One product copy's fit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProductFit {
    /// Index into [`Insert::placements`].
    pub placement: usize,
    pub color: FitColor,
    /// The smallest gap between the product and anything round it, in mm: 0 when they
    /// touch or overlap.
    pub clearance_mm: f64,
    /// The clearance recommended for the insert's board, which green needs.
    pub target_mm: f64,
    /// What a red product runs into; empty for green and yellow.
    pub clashes: Vec<Clash>,
}

/// The fit's colour in the editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FitColor {
    /// At least the recommended clearance.
    Green,
    /// Fits, but tighter than recommended.
    Yellow,
    /// Runs into something.
    Red,
}

/// What a red product runs into.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Clash {
    /// Board: the hole doesn't clear it, or a grid strip stands in the way.
    Board,
    /// Another product copy (index into [`Insert::placements`]), as when two share a
    /// merged hole.
    Product { placement: usize },
    /// A cradle leg, under the deck's long edge.
    Leg,
    /// The insert's edge: part of the product hangs off it.
    OffInsert,
}

impl Packaging {
    /// Every product copy's fit, in placement order (CLAUDE.md §7.3).
    pub fn fits(&self, design: &Design) -> Result<Vec<ProductFit>, GeometryError> {
        let insert = &self.insert;
        let products = placed_products((design, insert))?;
        let around = Surroundings::of((design, insert));
        let target = Material::starting_values(design.material.board).clearance_mm;
        let outlines: Vec<Outline> = products
            .iter()
            .filter_map(|product| {
                Some(Outline {
                    product,
                    ring: Ring::new(product.outline.clone())?,
                })
            })
            .collect();
        Ok(outlines
            .iter()
            .map(|outline| around.fit((outline, &outlines, target)))
            .collect())
    }
}

/// A product copy and its outline, ready for distance checks.
struct Outline<'a> {
    product: &'a PlacedProduct,
    ring: Ring,
}

/// A piece lying flat, seen from above: products stand in its holes.
struct FlatBoard {
    outline: Ring,
    holes: Vec<Ring>,
}

/// Everything round the products, in the insert's frame.
struct Surroundings {
    boards: Vec<FlatBoard>,
    /// Solid rectangles: upright strips (a clash with board) and cradle legs.
    blocks: Vec<(Ring, Clash)>,
    /// The insert's footprint.
    edge: Bounds,
}

impl Surroundings {
    fn of((design, insert): (&Design, &Insert)) -> Self {
        let thickness = design.material.thickness_mm;
        let mut boards = Vec::new();
        let mut blocks = Vec::new();
        for mount in &insert.mounts {
            let Some(piece) = insert.pieces.get(mount.piece) else {
                continue;
            };
            let [x, y, _] = mount.at_mm;
            match mount.stance {
                // A folding piece lies flat by its root panel; the rest folds away.
                Stance::Flat => {
                    let Some(panel) = piece.panels.first() else {
                        continue;
                    };
                    let to_insert = Transform::shift(Point::new(x, y));
                    let ring = |polygon: &Polygon| Ring::new(polygon.transformed(to_insert));
                    if let Some(outline) = ring(&panel.outline) {
                        let holes = panel.holes.iter().filter_map(ring).collect();
                        boards.push(FlatBoard { outline, holes });
                    }
                }
                // An upright strip: its length along its line, a board thick across it.
                stance => {
                    let outline = Polygon(piece.outline.flatten(CURVE_TOLERANCE_MM));
                    let Some(length) = outline.bounds().map(|bounds| bounds.length_mm()) else {
                        continue;
                    };
                    let half = thickness / 2.0;
                    let (xs, ys) = match stance {
                        Stance::UprightAlongY => ((x - half, x + half), (y, y + length)),
                        _ => ((x, x + length), (y - half, y + half)),
                    };
                    blocks.extend(rect((xs, ys)).map(|ring| (ring, Clash::Board)));
                }
            }
        }
        if let InsertSpec::CradlePlatform(cradle) = &design.insert
            && let Some(reach) = cradle.leg_reach_mm(&design.material)
        {
            let (length, width) = (insert.length_mm, insert.width_mm);
            for ys in [(0.0, reach), (width - reach, width)] {
                blocks.extend(rect(((0.0, length), ys)).map(|ring| (ring, Clash::Leg)));
            }
        }
        Self {
            boards,
            blocks,
            edge: Bounds::from_size((insert.length_mm, insert.width_mm)),
        }
    }

    /// One product's fit, among all the `outlines`.
    fn fit(&self, (outline, outlines, target): (&Outline, &[Outline], f64)) -> ProductFit {
        let (product, ring) = (outline.product, &outline.ring);
        let mut clashes = Vec::new();
        let mut clash = |clash: Clash| {
            if !clashes.contains(&clash) {
                clashes.push(clash);
            }
        };
        // The insert's edge: the product must lie inside its footprint.
        let b = ring.bounds;
        let edge = &self.edge;
        let mut clearance = (b.min.x - edge.min.x)
            .min(edge.max.x - b.max.x)
            .min(b.min.y - edge.min.y)
            .min(edge.max.y - b.max.y)
            .max(0.0);
        if clearance < FIT_SLACK_MM {
            clash(Clash::OffInsert);
        }
        for board in &self.boards {
            let gap = board_gap((board, ring, clearance.max(FIT_SLACK_MM)));
            if gap < FIT_SLACK_MM {
                clash(Clash::Board);
            }
            clearance = clearance.min(gap);
        }
        for (block, kind) in &self.blocks {
            let gap = ring.gap_to((block, clearance.max(FIT_SLACK_MM)));
            if gap < FIT_SLACK_MM {
                clash(*kind);
            }
            clearance = clearance.min(gap);
        }
        for other in outlines {
            let overlaps = other.product.placement != product.placement
                && ring.gap_to((&other.ring, FIT_SLACK_MM)) < FIT_SLACK_MM;
            if overlaps {
                clash(Clash::Product {
                    placement: other.product.placement,
                });
            }
        }
        let color = match (clashes.is_empty(), clearance >= target - FIT_SLACK_MM) {
            (false, _) => FitColor::Red,
            (true, true) => FitColor::Green,
            (true, false) => FitColor::Yellow,
        };
        ProductFit {
            placement: product.placement,
            color,
            clearance_mm: clearance,
            target_mm: target,
            clashes,
        }
    }
}

/// The gap from a product to a flat board's edges, up to `limit`: 0 when it crosses or
/// touches one, or lies on the board itself rather than in a hole. Keep `limit` at least
/// the contact threshold: a search that may look no further than 0 always finds 0.
fn board_gap((board, product, limit): (&FlatBoard, &Ring, f64)) -> f64 {
    let point = product.first();
    let on_board = board.outline.polygon.contains(point)
        && !board
            .holes
            .iter()
            .any(|hole| hole.bounds.gap_to(product.bounds) <= 0.0 && hole.polygon.contains(point));
    if on_board {
        return 0.0;
    }
    // Nearest rings first (the product's own hole), so the far ones are skipped.
    let mut near: Vec<(f64, &Ring)> = std::iter::once(&board.outline)
        .chain(&board.holes)
        .map(|ring| (ring.bounds.gap_to(product.bounds), ring))
        .filter(|&(gap, _)| gap < limit)
        .collect();
    near.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut best = limit;
    for (gap, ring) in near {
        if gap >= best {
            break;
        }
        best = best.min(product.line_gap((ring, best)));
    }
    best
}

/// The rectangle between two x values and two y values, as a ring.
fn rect(((x0, x1), (y0, y1)): ((f64, f64), (f64, f64))) -> Option<Ring> {
    Ring::new(
        Bounds {
            min: Point::new(x0, y0),
            max: Point::new(x1, y1),
        }
        .to_polygon(),
    )
}
