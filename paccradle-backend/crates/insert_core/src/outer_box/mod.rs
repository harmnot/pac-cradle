//! Outer boxes (CLAUDE.md §6.3): the box the insert goes in, each cut as one piece that
//! folds.
//!
//! - Inside size: the insert's footprint plus the box's clearance on each side, and the
//!   tallest of the insert and the products plus the clearance as headroom. A box with a
//!   fixed inside size (one the brand already stocks) keeps it; the insert generators
//!   already sized the insert to fit, and here the products must not be taller than it.
//! - Every style needs a smallest inside size for its own folds and tabs. A box sized
//!   round the insert grows to it; a fixed box smaller than it is refused.
//! - Box pieces are drawn from the inside: the box's inside faces the viewer, so every
//!   crease is a valley fold and print goes on the underside of the drawing.
//! - [`OuterBox::mounts`] place the box in the insert's frame. The insert sits in the
//!   middle of the inside floor, which is at z = 0.
//!
//! Styles: `mailer` (FEFCO 0427 style) and `shipping` (FEFCO 0201).

mod mailer;
mod shipping;

use serde::{Deserialize, Serialize};

use crate::design::{BoxSize, BoxSpec, BoxStyle, Design};
use crate::error::GenerateError;
use crate::insert::{Insert, Mount};
use crate::material::Material;
use crate::piece::Piece;

/// Slack when checking heights against a fixed box, for floating-point noise.
const FIT_SLACK_MM: f64 = 1e-6;

/// A generated outer box.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OuterBox {
    pub style: BoxStyle,
    pub pieces: Vec<Piece>,
    /// The room inside: the insert and the products go in here.
    pub inside_mm: BoxSize,
    /// The box's overall size, board included (what a carrier measures).
    pub outside_mm: BoxSize,
    /// Where every cut copy of every piece goes, in the insert's frame (see
    /// [`Insert::mounts`]): one entry per copy.
    pub mounts: Vec<Mount>,
}

/// One box style's construction (CLAUDE.md §6.3).
trait BoxConstruction {
    /// Smallest inside size its folds, flaps and tabs work at, in this board.
    fn min_inside(&self, material: &Material) -> BoxSize;

    /// Cuts the box with this inside size. The box's frame has the inside's corner
    /// nearest (0, 0) at the origin and the inside floor at z = 0.
    fn build(&self, plan: (BoxSize, &Material)) -> Result<BuiltBox, GenerateError>;
}

/// A cut box, in the box's own frame.
struct BuiltBox {
    piece: Piece,
    outside_mm: BoxSize,
    mount: Mount,
}

impl BoxSpec {
    /// Generates the box round an insert generated from `design`.
    pub(crate) fn generate(
        &self,
        (design, insert): (&Design, &Insert),
    ) -> Result<OuterBox, GenerateError> {
        let construction: &dyn BoxConstruction = match self.style {
            BoxStyle::Mailer => &mailer::Mailer,
            BoxStyle::Shipping => &shipping::Shipping,
        };
        let min = construction.min_inside(&self.material);
        let inside = self.inside_size((design, insert, min))?;
        let built = construction.build((inside, &self.material))?;
        // The insert sits in the middle of the inside floor.
        let corner = [
            (insert.length_mm - inside.length_mm) / 2.0,
            (insert.width_mm - inside.width_mm) / 2.0,
        ];
        let [x, y, z] = built.mount.at_mm;
        Ok(OuterBox {
            style: self.style,
            pieces: vec![built.piece],
            inside_mm: inside,
            outside_mm: built.outside_mm,
            mounts: vec![Mount {
                at_mm: [x + corner[0], y + corner[1], z],
                ..built.mount
            }],
        })
    }

    /// The inside size: the fixed one if the products fit it, or the insert plus the
    /// clearance, grown to the construction's smallest size.
    fn inside_size(
        &self,
        (design, insert, min): (&Design, &Insert, BoxSize),
    ) -> Result<BoxSize, GenerateError> {
        let tallest = design
            .products
            .iter()
            .map(|product| product.height_mm)
            .fold(insert.height_mm, f64::max);
        let Some(fixed) = self.inside_mm else {
            let gaps = 2.0 * self.material.clearance_mm;
            return Ok(BoxSize {
                length_mm: (insert.length_mm + gaps).max(min.length_mm),
                width_mm: (insert.width_mm + gaps).max(min.width_mm),
                height_mm: (tallest + self.material.clearance_mm).max(min.height_mm),
            });
        };
        if tallest > fixed.height_mm + FIT_SLACK_MM {
            return Err(GenerateError::TooTallForBox {
                needed_height_mm: tallest,
                room_height_mm: fixed.height_mm,
            });
        }
        let big_enough = fixed.length_mm >= min.length_mm
            && fixed.width_mm >= min.width_mm
            && fixed.height_mm >= min.height_mm;
        match big_enough {
            true => Ok(fixed),
            false => Err(GenerateError::BoxTooSmall {
                min_length_mm: min.length_mm,
                min_width_mm: min.width_mm,
                min_height_mm: min.height_mm,
            }),
        }
    }
}
