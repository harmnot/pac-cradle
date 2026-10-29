//! Cradle products (M1-A5 notes): a product no taller than the deck sinks below it, and a
//! product standing over a leg runs into it.
//!
//! The legs run under both long edges of the deck, in to the riser's inner face. The
//! generator keeps its own rows clear of them; a placement from the editor may not be.

use super::{Check, Context, Part, Rule, SLACK_MM, Spot, Warning, union};
use crate::design::InsertSpec;
use crate::error::GeometryError;
use crate::geometry::{Bounds, Point};
use crate::insert::placed_products;

pub(super) struct CradleProducts;

impl Check for CradleProducts {
    fn run(&self, context: &Context) -> Result<Vec<Warning>, GeometryError> {
        let (design, insert) = (context.design, &context.packaging.insert);
        let InsertSpec::CradlePlatform(cradle) = &design.insert else {
            return Ok(Vec::new());
        };
        let products = placed_products((design, insert))?;
        let Some((piece, shift)) = context.flat_insert_piece() else {
            return Ok(Vec::new());
        };
        // Into the piece's drawing, where the spots point.
        let on_piece = |bounds: Bounds| Bounds {
            min: bounds.min + shift,
            max: bounds.max + shift,
        };
        let spot = |(area, placements): (Option<Bounds>, Vec<usize>)| Spot {
            part: Part::Insert,
            piece: Some(piece),
            area_mm: area,
            placements,
        };
        let mut warnings = Vec::new();

        for (index, product) in design.products.iter().enumerate() {
            if product.height_mm > cradle.deck_height_mm + SLACK_MM {
                continue;
            }
            let copies: Vec<_> = products
                .iter()
                .filter(|copy| copy.product == index)
                .collect();
            let area = copies.iter().fold(None, |area, copy| {
                union(area, copy.outline.bounds().map(on_piece))
            });
            warnings.push(Warning {
                rule: Rule::ProductBelowDeck {
                    product_id: product.id.clone(),
                    height_mm: product.height_mm,
                    deck_height_mm: cradle.deck_height_mm,
                },
                at: spot((area, copies.iter().map(|copy| copy.placement).collect())),
            });
        }

        let Some(reach) = cradle.leg_reach_mm(&design.material) else {
            return Ok(warnings);
        };
        // The legs' strips under the deck, in the insert's frame.
        let legs = [
            Bounds {
                min: Point::ORIGIN,
                max: Point::new(insert.length_mm, reach),
            },
            Bounds {
                min: Point::new(0.0, insert.width_mm - reach),
                max: Point::new(insert.length_mm, insert.width_mm),
            },
        ];
        for copy in &products {
            let Some(bounds) = copy.outline.bounds() else {
                continue;
            };
            let over_leg = legs.iter().any(|leg| overlap((bounds, *leg)));
            if over_leg {
                warnings.push(Warning {
                    rule: Rule::ProductOverLeg {
                        product_id: design.products[copy.product].id.clone(),
                    },
                    at: spot((Some(on_piece(bounds)), vec![copy.placement])),
                });
            }
        }
        Ok(warnings)
    }
}

/// True when the boxes share more than an edge. A leg runs the deck's whole length, so a
/// product's box reaching into the leg's strip means the product does.
fn overlap((a, b): (Bounds, Bounds)) -> bool {
    a.min.x < b.max.x - SLACK_MM
        && b.min.x < a.max.x - SLACK_MM
        && a.min.y < b.max.y - SLACK_MM
        && b.min.y < a.max.y - SLACK_MM
}
