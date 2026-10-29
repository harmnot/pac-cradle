//! Cutout card (CLAUDE.md §6.2): a flat card with a product-shaped hole for every copy.
//!
//! - Hole: the product's outline plus the board's clearance, with an optional finger notch.
//! - Card: when the box has a fixed inside size, that size minus the box's clearance on
//!   each side. Otherwise it grows around the holes, keeping `margin_mm` to every edge.

use super::hole::{HoleStyle, ProductHole};
use super::layout::{Insets, LayoutRequest};
use super::{GenerateInsert, Insert, Mount, Stance, fixed_room, flat_piece};
use crate::design::{CutoutCardSpec, Design};
use crate::error::GenerateError;
use crate::folding::one_region;
use crate::geometry::Area;
use crate::material::Material;

/// Name of the piece in files and the editor.
const PIECE_NAME: &str = "card";

impl GenerateInsert for CutoutCardSpec {
    fn generate(&self, design: &Design) -> Result<Insert, GenerateError> {
        let style = self.hole_style(&design.material);
        let holes = design
            .products
            .iter()
            .map(|product| ProductHole::new(product, style))
            .collect::<Result<Vec<_>, _>>()?;
        let fixed_size = fixed_room(design.outer_box.as_ref());
        let insets = Insets::even(self.margin_mm);
        let layout = LayoutRequest {
            products: &design.products,
            holes: &holes,
            placements: &self.placements,
            align: self.align,
            gap_mm: design.material.min_web_mm(),
            space: fixed_size.map(|size| insets.room(size)),
        }
        .lay_out();
        let (card, layout) = match fixed_size {
            Some(size) => (layout.fixed_board((size, insets))?, layout),
            None => layout.grown_board(insets),
        };

        let board = Area::from_polygon(&card.to_polygon())?;
        let cut = board.difference(&Area::union_all(&layout.holes)?)?;
        let region = one_region((PIECE_NAME, cut))?;
        let piece = flat_piece((PIECE_NAME, region)).ok_or(GenerateError::FallsApart {
            piece: PIECE_NAME,
            parts: 0,
        })?;
        Ok(Insert {
            pieces: vec![piece],
            length_mm: card.length_mm(),
            width_mm: card.width_mm(),
            height_mm: design.material.thickness_mm,
            placements: layout.placements,
            mounts: vec![Mount {
                piece: 0,
                stance: Stance::Flat,
                at_mm: [0.0, 0.0, 0.0],
            }],
        })
    }
}

impl CutoutCardSpec {
    /// How the card cuts each product's hole.
    pub(super) fn hole_style(&self, material: &Material) -> HoleStyle {
        HoleStyle {
            clearance_mm: material.clearance_mm,
            finger_notches: self.finger_notches,
        }
    }
}
