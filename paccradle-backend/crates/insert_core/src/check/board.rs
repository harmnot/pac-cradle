//! Board settings (CLAUDE.md §6.4, §6.5): a thickness outside the usual range for the
//! board, and slots that don't fit the board they take.
//!
//! Every slot is cut a board plus the slot tolerance wide, so it matches the board when
//! the tolerance is sensible ([`SLOT_PLAY_MM`]). Slots take board in the partition grid
//! (crossing strips), the cradle (lock tabs) and the mailer (lock tabs). The card has
//! none, and the shipping box's flap slots take no board.

use super::{Check, Context, Part, Rule, SLACK_MM, SLOT_PLAY_MM, Spot, Warning};
use crate::design::{BoxStyle, InsertSpec};
use crate::error::GeometryError;
use crate::material::Material;

pub(super) struct BoardSettings;

impl Check for BoardSettings {
    fn run(&self, context: &Context) -> Result<Vec<Warning>, GeometryError> {
        let design = context.design;
        let mut parts = vec![(Part::Insert, &design.material)];
        parts.extend(
            design
                .outer_box
                .as_ref()
                .map(|spec| (Part::OuterBox, &spec.material)),
        );
        let mut warnings: Vec<Warning> = parts
            .iter()
            .filter_map(|&(part, material)| unusual_thickness((part, material)))
            .collect();
        for piece in context.pieces() {
            if has_slots((context, piece.part, piece.index)) {
                warnings.extend(slot_fit(piece.material).map(|rule| Warning {
                    rule,
                    at: piece.spot(),
                }));
            }
        }
        Ok(warnings)
    }
}

/// A warning when the board is thicker or thinner than usual for its kind.
fn unusual_thickness((part, material): (Part, &Material)) -> Option<Warning> {
    let typical = material.board.typical_thickness();
    (!typical.contains(material.thickness_mm)).then(|| Warning {
        rule: Rule::UnusualThickness {
            board: material.board,
            thickness_mm: material.thickness_mm,
            typical,
        },
        at: Spot {
            part,
            piece: None,
            area_mm: None,
            placements: Vec::new(),
        },
    })
}

/// True for the pieces whose slots take board.
fn has_slots((context, part, index): (&Context, Part, usize)) -> bool {
    match part {
        Part::Insert => match context.design.insert {
            InsertSpec::PartitionGrid(_) => true,
            InsertSpec::CradlePlatform(_) => index == 0,
            InsertSpec::CutoutCard(_) => false,
        },
        Part::OuterBox => context
            .design
            .outer_box
            .is_some_and(|spec| spec.style == BoxStyle::Mailer && index == 0),
    }
}

/// The rule broken when the slots are too tight or too loose for the board.
fn slot_fit(material: &Material) -> Option<Rule> {
    let play = material.slot_tolerance_mm;
    let slot_mm = material.thickness_mm + play;
    let board_mm = material.thickness_mm;
    if play < SLOT_PLAY_MM.min - SLACK_MM {
        return Some(Rule::SlotTooTight { slot_mm, board_mm });
    }
    (play > SLOT_PLAY_MM.max + SLACK_MM).then_some(Rule::SlotTooLoose { slot_mm, board_mm })
}
