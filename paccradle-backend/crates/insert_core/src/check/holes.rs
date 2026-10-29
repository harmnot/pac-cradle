//! Hole spacing (CLAUDE.md §6.5): holes closer than a web (3 mm or 3× the board) to each
//! other or to the piece's edge, and product holes closer than 2× the board to a crease.
//!
//! Product holes come from the placements, one per copy, built the way the generator
//! builds them; the piece's own holes may have merged them or lost them off the edge.
//! The piece's other holes are lock slots. They sit beside their creases on purpose (the
//! cradle's legs, the mailer's inner end walls), so the crease rule leaves them out.

use super::{
    CREASE_CLEARANCE_BOARDS, Check, Context, Part, PieceRef, Rule, SLACK_MM, Warning, lines, points,
};
use crate::error::GeometryError;
use crate::geometry::{Bounds, Line, Point, Ring, Transform, edges_gap};
use crate::insert::{PlacedProduct, placed_products};

pub(super) struct HoleSpacing;

/// A hole in a piece.
struct Hole {
    ring: Ring,
    /// The product copy it holds (index into `Insert::placements`); `None` for a lock slot.
    placement: Option<usize>,
}

impl Check for HoleSpacing {
    fn run(&self, context: &Context) -> Result<Vec<Warning>, GeometryError> {
        let products = placed_products((context.design, &context.packaging.insert))?;
        let flat = context.flat_insert_piece();
        let mut warnings = Vec::new();
        for piece in context.pieces() {
            let mut holes = match flat {
                Some((index, shift)) if piece.index == index && piece.part == Part::Insert => {
                    product_holes((&products, shift))
                }
                _ => Vec::new(),
            };
            let slots = slots((piece, &holes));
            holes.extend(slots);
            warnings.extend(spacing((piece, &holes)));
        }
        Ok(warnings)
    }
}

/// Every product copy's hole, moved into the piece's drawing.
fn product_holes((products, shift): (&[PlacedProduct], Point)) -> Vec<Hole> {
    let to_piece = Transform::shift(shift);
    products
        .iter()
        .filter_map(|product| Some((product.placement, product.hole.as_ref()?)))
        .flat_map(|(placement, hole)| {
            hole.regions().iter().filter_map(move |region| {
                let ring = Ring::new(region.outer.transformed(to_piece))?;
                Some(Hole {
                    ring,
                    placement: Some(placement),
                })
            })
        })
        .collect()
}

/// The piece's holes that hold no product: its lock slots.
fn slots((piece, products): (PieceRef, &[Hole])) -> Vec<Hole> {
    piece
        .piece
        .holes
        .iter()
        .filter_map(|path| Ring::new(points(path)))
        .filter(|ring| {
            products
                .iter()
                .all(|product| ring.gap_to((&product.ring, SLACK_MM)) >= SLACK_MM)
        })
        .map(|ring| Hole {
            ring,
            placement: None,
        })
        .collect()
}

/// Every spacing rule on one piece's holes.
fn spacing((piece, holes): (PieceRef, &[Hole])) -> Vec<Warning> {
    let web = piece.material.min_web_mm();
    let near_crease = CREASE_CLEARANCE_BOARDS * piece.material.thickness_mm;
    let at = |(area, placements): (Bounds, Vec<usize>)| {
        piece.spot().area(Some(area)).placements(placements)
    };
    let mut warnings = Vec::new();
    for (index, hole) in holes.iter().enumerate() {
        for other in &holes[index + 1..] {
            let gap = hole.ring.gap_to((&other.ring, web));
            if gap < web - SLACK_MM {
                let placements = hole.placement.into_iter().chain(other.placement).collect();
                warnings.push(Warning {
                    rule: Rule::HolesTooClose {
                        gap_mm: gap,
                        min_mm: web,
                    },
                    at: at((hole.ring.bounds.union(other.ring.bounds), placements)),
                });
            }
        }
    }
    if let Some(outline) = Ring::new(points(&piece.piece.outline)) {
        for hole in holes {
            let gap = edge_gap((&hole.ring, &outline, web));
            if gap < web - SLACK_MM {
                warnings.push(Warning {
                    rule: Rule::HoleNearEdge {
                        gap_mm: gap,
                        min_mm: web,
                    },
                    at: at((hole.ring.bounds, hole.placement.into_iter().collect())),
                });
            }
        }
    }
    let creases: Vec<Vec<(Line, Bounds)>> = piece
        .piece
        .creases
        .iter()
        .map(|crease| {
            lines(crease)
                .into_iter()
                .map(|line| (line, line.bounds()))
                .collect()
        })
        .collect();
    for hole in holes.iter().filter(|hole| hole.placement.is_some()) {
        for crease in &creases {
            let gap = edges_gap((&hole.ring.edges, crease, near_crease));
            if gap < near_crease - SLACK_MM {
                warnings.push(Warning {
                    rule: Rule::HoleNearCrease {
                        gap_mm: gap,
                        min_mm: near_crease,
                    },
                    at: at((hole.ring.bounds, hole.placement.into_iter().collect())),
                });
            }
        }
    }
    warnings
}

/// The gap from a hole to the piece's outline, up to `limit`: 0 when the hole crosses it
/// or lies outside the piece.
fn edge_gap((hole, outline, limit): (&Ring, &Ring, f64)) -> f64 {
    if !outline.polygon.contains(hole.first()) {
        return 0.0;
    }
    edges_gap((&hole.edges, &outline.edges, limit))
}
