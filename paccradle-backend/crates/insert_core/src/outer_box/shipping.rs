//! Shipping box, FEFCO 0201 (regular slotted container): four walls in a row with a
//! joint flap, and four flaps at the bottom and four at the top.
//!
//! - The back wall is the root, standing upright along x. The right, front and left walls
//!   follow round the box. The joint flap on the left wall's far end is glued or taped
//!   outside the back wall (the one joint every 0201 has), which keeps the inside clear.
//! - The score lines run straight across, so every wall is the same height between its
//!   creases.
//! - The flaps on the two longer walls ("major" flaps) meet in the middle with the
//!   tolerance between them. The flaps on the shorter walls ("minor" flaps) fold first
//!   and lie inside. The major flaps fold over them: past the score, the board runs on
//!   one board thickness in the wall's plane (the bend, see [`Hinge::offset_mm`]) before
//!   the flap turns, so wall, bend and flap stay one board.
//! - The bend uses up a board of each major flap, so every flap is a board deeper than
//!   it reaches. The minor flaps are as deep, so all the tips line up, except in a box so
//!   nearly square that they'd run into each other: there they stop the tolerance apart.
//! - The slots between flaps are a board plus the tolerance wide.
//!
//! In the box's frame (inside from (0, 0, 0) to (L, W, H); t the board, g its slot
//! tolerance), the middle planes sit at:
//! - walls: back y = −t/2, right x = L + t/2, front y = W + t/2, left x = −t/2; the
//!   joint flap at y = −1.5t. The back wall's free end stops at the left wall's inner
//!   face;
//! - every crease to a flap at z = −t/2 or H + t/2: the minor flaps lie there, the major
//!   flaps a board further out, at z = −1.5t and H + 1.5t, joined to their walls by the
//!   bends.
//!
//! [`Hinge::offset_mm`]: crate::Hinge::offset_mm

use super::{BoxConstruction, BuiltBox};
use crate::design::BoxSize;
use crate::error::GenerateError;
use crate::folding::{BuiltPiece, FoldingPiece, Joint, along_x, along_y, rect};
use crate::geometry::{Area, Line};
use crate::insert::{Mount, Stance};
use crate::material::Material;
use crate::piece::FoldDirection;

/// Name of the piece in files and the editor.
const PIECE_NAME: &str = "shipping box";
/// Widest joint flap; narrower on a short box (half the length).
const JOINT_FLAP_MM: f64 = 30.0;
/// Every crease folds a quarter turn.
const FOLD_DEG: f64 = 90.0;
/// Back, right, front, left.
const WALLS: usize = 4;

/// The shipping box construction.
pub(super) struct Shipping;

/// Fold order for the animation, the order the box is put together in.
#[derive(Debug, Clone, Copy)]
enum Stage {
    /// The walls round the box, and the joint flap onto the back wall.
    Walls,
    BottomMinor,
    BottomMajor,
    TopMinor,
    TopMajor,
}

impl BoxConstruction for Shipping {
    /// Any box a web across in every direction, and long enough for a joint flap of two
    /// webs.
    fn min_inside(&self, material: &Material) -> BoxSize {
        let web = material.min_web_mm();
        BoxSize {
            length_mm: 2.0 * web,
            width_mm: web,
            height_mm: web,
        }
    }

    fn build(&self, (inside, material): (BoxSize, &Material)) -> Result<BuiltBox, GenerateError> {
        let plan = Plan {
            t: material.thickness_mm,
            g: material.slot_tolerance_mm,
            inside,
        };
        let built = plan.piece()?;
        let t = plan.t;
        Ok(BuiltBox {
            piece: built.piece,
            outside_mm: BoxSize {
                length_mm: inside.length_mm + 2.0 * t,
                width_mm: inside.width_mm + 3.0 * t,
                height_mm: inside.height_mm + 4.0 * t,
            },
            // The drawing runs along x with y down; standing upright, drawn y becomes −z.
            mount: Mount {
                piece: 0,
                stance: Stance::UprightAlongX,
                at_mm: [built.flat_min.x, -t / 2.0, -built.flat_min.y],
            },
        })
    }
}

/// Where everything goes. The walls are drawn in a row along x from the back wall's free
/// end, as the back wall stands (x as in the box); drawn y is −z.
struct Plan {
    t: f64,
    g: f64,
    inside: BoxSize,
}

impl Plan {
    /// Where wall `index`'s middle plane would meet the previous wall's, along the row:
    /// the line its flaps' slot is centred on. Index 4 is the end of the left wall's
    /// middle-plane span, just short of the joint flap.
    fn corner(&self, index: usize) -> f64 {
        let (t, length, width) = (self.t, self.inside.length_mm, self.inside.width_mm);
        // Spans between the walls' middle planes: L + t along x, W + t along y.
        let spans = [length + t, width + t, length + t, width + t];
        -t / 2.0 + spans.iter().take(index).sum::<f64>()
    }

    /// True for the walls whose flaps meet in the middle: the two longer walls.
    fn is_major(&self, wall: usize) -> bool {
        let along_x = wall.is_multiple_of(2);
        along_x == (self.inside.length_mm >= self.inside.width_mm)
    }

    /// The score lines to the top and bottom flaps, straight across every wall, as drawn
    /// y values (top first).
    fn creases_y(&self) -> (f64, f64) {
        let (t, height) = (self.t, self.inside.height_mm);
        (-(height + t / 2.0), t / 2.0)
    }

    /// How deep the wall's flaps are past the score. Two flaps on facing walls, whose
    /// middle planes are a side plus a board apart, reach half that less half the
    /// tolerance; the major flaps also need a board for their bend.
    fn flap_depth(&self, wall: usize) -> f64 {
        let (t, g) = (self.t, self.g);
        let (length, width) = (self.inside.length_mm, self.inside.width_mm);
        let reach = |side: f64| (side + t - g) / 2.0;
        let major = reach(length.min(width)) + t;
        match self.is_major(wall) {
            true => major,
            false => major.min(reach(length.max(width))),
        }
    }

    fn joint(&self, (parent, axis): (usize, Line), stage: Stage) -> Joint {
        Joint {
            parent,
            axis,
            deg: FOLD_DEG,
            direction: FoldDirection::Valley,
            offset_mm: 0.0,
            stage: stage as u32,
        }
    }

    /// The whole flat piece.
    fn piece(&self) -> Result<BuiltPiece, GenerateError> {
        let (t, g) = (self.t, self.g);
        let creases = self.creases_y();
        // The back wall starts at the left wall's inner face, the left wall runs on to
        // the joint flap outside the back wall.
        let starts = [0.0, self.corner(1), self.corner(2), self.corner(3)];
        let ends = [
            self.corner(1),
            self.corner(2),
            self.corner(3),
            self.corner(4) + t,
        ];
        let wall_area =
            |wall: usize| Area::from_polygon(&rect(((starts[wall], ends[wall]), creases)));

        let mut piece = FoldingPiece::new((PIECE_NAME, wall_area(0)?));
        let mut walls = vec![0];
        for wall in 1..WALLS {
            let axis = along_y((starts[wall], creases));
            let index = piece.fold((
                wall_area(wall)?,
                self.joint((walls[wall - 1], axis), Stage::Walls),
            ));
            walls.push(index);
        }
        let joint_flap_width = JOINT_FLAP_MM.min(self.inside.length_mm / 2.0);
        let joint_ys = (creases.0 + g, creases.1 - g);
        let joint_x = ends[WALLS - 1];
        piece.fold((
            Area::from_polygon(&rect(((joint_x, joint_x + joint_flap_width), joint_ys)))?,
            self.joint(
                (walls[WALLS - 1], along_y((joint_x, joint_ys))),
                Stage::Walls,
            ),
        ));

        let slot = t + g;
        for (wall, &index) in walls.iter().enumerate() {
            let depth = self.flap_depth(wall);
            let across = (
                self.corner(wall) + slot / 2.0,
                self.corner(wall + 1) - slot / 2.0,
            );
            let (top, bottom) = creases;
            // Major flaps bend round the minor ones and turn a board past the score.
            let (bottom_stage, top_stage, offset_mm) = match self.is_major(wall) {
                true => (Stage::BottomMajor, Stage::TopMajor, t),
                false => (Stage::BottomMinor, Stage::TopMinor, 0.0),
            };
            for (crease, tip, stage) in [
                (bottom, bottom + depth, bottom_stage),
                (top, top - depth, top_stage),
            ] {
                piece.fold((
                    Area::from_polygon(&rect((across, (crease, tip))))?,
                    Joint {
                        offset_mm,
                        ..self.joint((index, along_x((crease, across))), stage)
                    },
                ));
            }
        }
        piece.build()
    }
}
