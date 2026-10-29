//! Panel and piece sizes (CLAUDE.md §6.5): a panel whose holes take more than
//! [`MAX_CUT_OUT_SHARE`] of it gets weak, and a piece must fit the chosen sheet or
//! cutting mat (either way round).

use super::{Check, Context, MAX_CUT_OUT_SHARE, Rule, SLACK_MM, SheetSize, Warning, points};
use crate::error::GeometryError;
use crate::piece::Panel;

pub(super) struct Panels;

impl Check for Panels {
    fn run(&self, context: &Context) -> Result<Vec<Warning>, GeometryError> {
        let mut warnings = Vec::new();
        for piece in context.pieces() {
            for panel in &piece.piece.panels {
                let share = cut_out_share(panel);
                if share > MAX_CUT_OUT_SHARE + 1e-9 {
                    warnings.push(Warning {
                        rule: Rule::WeakPanel {
                            cut_out_share: share,
                            max_share: MAX_CUT_OUT_SHARE,
                        },
                        at: piece.spot().area(panel.outline.bounds()),
                    });
                }
            }
            let Some(sheet) = context.options.sheet else {
                continue;
            };
            let Some(bounds) = points(&piece.piece.outline).bounds() else {
                continue;
            };
            let (length_mm, width_mm) = (bounds.length_mm(), bounds.width_mm());
            if !fits(((length_mm, width_mm), sheet)) {
                warnings.push(Warning {
                    rule: Rule::TooBigForSheet {
                        length_mm,
                        width_mm,
                        sheet,
                    },
                    at: piece.spot(),
                });
            }
        }
        Ok(warnings)
    }
}

/// How much of the panel its holes take, from 0 to 1.
fn cut_out_share(panel: &Panel) -> f64 {
    let gross = panel.outline.signed_area_mm2().abs();
    let holes: f64 = panel
        .holes
        .iter()
        .map(|hole| hole.signed_area_mm2().abs())
        .sum();
    match gross > 0.0 {
        true => holes / gross,
        false => 0.0,
    }
}

/// True when a piece of this size fits the sheet, either way round.
fn fits(((length, width), sheet): ((f64, f64), SheetSize)) -> bool {
    let within =
        |(a, b): (f64, f64)| a <= sheet.length_mm + SLACK_MM && b <= sheet.width_mm + SLACK_MM;
    within((length, width)) || within((width, length))
}
