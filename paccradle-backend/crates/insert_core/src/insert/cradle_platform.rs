//! Cradle platform (CLAUDE.md §6.2): a raised deck with a hole for every product,
//! standing on two folded legs, cut as one piece. Nothing is glued.
//!
//! - Deck: holes and size work as on the cutout card. A hole is the product's outline
//!   plus the clearance. In a box with a fixed inside size, the deck fills it; otherwise it
//!   grows round the holes. Until side profiles arrive (v1, §7.3) a product is a straight
//!   prism, so its outline at deck height is the top-view outline.
//! - Legs, one along each long edge (along x): the board folds down into a wall, in
//!   along the floor into a foot, and back up into a riser. Lock tabs on the riser's top
//!   edge push up through slots in the deck and end flush with its top, which closes
//!   the leg into a tube. All three folds are 90° and turn the same way.
//! - The ends (along y) stay open. The box walls stop the deck sliding.
//!
//! Lengths between creases are middle-plane lengths (see [`crate::piece`]). The deck's
//! creases sit half a board in from its long edges, so the walls' outer faces line up
//! with the footprint. The feet rest on the floor, and the deck's top is at
//! `deck_height_mm`.

use super::hole::{HoleStyle, ProductHole};
use super::layout::{Insets, LayoutRequest};
use super::{GenerateInsert, Insert, Mount, Stance, fixed_room};
use crate::design::{CradlePlatformSpec, Design};
use crate::error::GenerateError;
use crate::folding::{BuiltPiece, FoldingPiece, Joint, LockTabs, along_x, one_region, rect};
use crate::geometry::{Area, Bounds, Polygon};
use crate::material::Material;
use crate::piece::FoldDirection;

/// Name of the piece in files and the editor.
const PIECE_NAME: &str = "cradle";
/// Name of the deck panel, when the holes cut it in two.
const DECK_NAME: &str = "deck";
/// Every leg crease folds a quarter turn.
const LEG_FOLD_DEG: f64 = 90.0;
/// Panels per leg: wall, foot, riser.
const LEG_PANELS: usize = 3;

impl GenerateInsert for CradlePlatformSpec {
    fn generate(&self, design: &Design) -> Result<Insert, GenerateError> {
        let rules = LegRules::new((&design.material, self.deck_height_mm))?;
        let style = self.hole_style(&design.material);
        let holes = design
            .products
            .iter()
            .map(|product| ProductHole::new(product, style))
            .collect::<Result<Vec<_>, _>>()?;
        let insets = Insets {
            x_mm: self.margin_mm,
            y_mm: self.margin_mm.max(rules.hole_inset_mm()),
        };
        let fixed_size = fixed_room(design.outer_box.as_ref());
        let layout = LayoutRequest {
            products: &design.products,
            holes: &holes,
            placements: &self.placements,
            align: self.align,
            gap_mm: design.material.min_web_mm(),
            space: fixed_size.map(|size| insets.room(size)),
        }
        .lay_out();
        let (deck, layout) = match fixed_size {
            Some(size) => (layout.fixed_board((size, insets))?, layout),
            None => layout.grown_board(insets),
        };

        let tabs = rules
            .tabs
            .spans(deck.length_mm())
            .ok_or_else(|| rules.too_small())?;
        let piece = rules.piece((deck, &layout.holes, &tabs))?;
        let corner = piece.flat_min;
        Ok(Insert {
            pieces: vec![piece.piece],
            length_mm: deck.length_mm(),
            width_mm: deck.width_mm(),
            height_mm: self.deck_height_mm,
            placements: layout.placements,
            mounts: vec![Mount {
                piece: 0,
                stance: Stance::Flat,
                at_mm: [
                    corner.x,
                    corner.y,
                    self.deck_height_mm - design.material.thickness_mm,
                ],
            }],
        })
    }
}

impl CradlePlatformSpec {
    /// How the deck cuts each product's hole: the outline plus the clearance.
    pub(super) fn hole_style(&self, material: &Material) -> HoleStyle {
        HoleStyle {
            clearance_mm: material.clearance_mm,
            finger_notches: None,
        }
    }

    /// How far in from each long edge of the deck the legs reach once folded: to the
    /// riser's inner face. A product must stand clear of it. `None` when the deck is too
    /// low for legs.
    pub(crate) fn leg_reach_mm(&self, material: &Material) -> Option<f64> {
        let rules = LegRules::new((material, self.deck_height_mm)).ok()?;
        Some(rules.riser_in_mm() + rules.thickness_mm / 2.0)
    }
}

/// A long edge of the deck, and the leg that folds down from it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    /// The edge at y = 0 (top of the drawing).
    Back,
    /// The edge at the largest y.
    Front,
}

impl Side {
    const BOTH: [Self; 2] = [Self::Back, Self::Front];

    /// Which way is out of the deck across this edge, along y.
    fn outward(self) -> f64 {
        match self {
            Self::Back => -1.0,
            Self::Front => 1.0,
        }
    }

    /// Where this edge's crease sits: half a board in from the footprint's edge.
    fn crease_y(self, (deck, thickness_mm): (Bounds, f64)) -> f64 {
        match self {
            Self::Back => deck.min.y + thickness_mm / 2.0,
            Self::Front => deck.max.y - thickness_mm / 2.0,
        }
    }
}

/// Leg sizes, from the board and the deck height.
#[derive(Debug, Clone, Copy)]
struct LegRules {
    thickness_mm: f64,
    /// From the deck's crease down to the floor.
    wall_mm: f64,
    /// Along the floor, from the wall in to the riser: half a slot plus a web, so the
    /// lock slot keeps a full web of board between it and the deck's crease.
    foot_mm: f64,
    /// From the floor up to the deck's underside.
    riser_mm: f64,
    slot_width_mm: f64,
    /// Narrowest board beside a slot, a hole or an edge (CLAUDE.md §6.5).
    web_mm: f64,
    /// The riser's lock tabs, along the deck's length.
    tabs: LockTabs,
}

impl LegRules {
    /// The legs for a deck whose top is `deck_height_mm` above the floor, or
    /// [`GenerateError::DeckTooSmall`] when the riser would be shorter than a web.
    fn new((material, deck_height_mm): (&Material, f64)) -> Result<Self, GenerateError> {
        let t = material.thickness_mm;
        let slot_width_mm = t + material.slot_tolerance_mm;
        let web_mm = material.min_web_mm();
        let rules = Self {
            thickness_mm: t,
            wall_mm: deck_height_mm - t,
            foot_mm: slot_width_mm / 2.0 + web_mm,
            riser_mm: deck_height_mm - 1.5 * t,
            slot_width_mm,
            web_mm,
            tabs: LockTabs {
                web_mm,
                tolerance_mm: material.slot_tolerance_mm,
            },
        };
        match rules.riser_mm >= web_mm {
            true => Ok(rules),
            false => Err(rules.too_small()),
        }
    }

    fn too_small(&self) -> GenerateError {
        GenerateError::DeckTooSmall {
            min_length_mm: self.tabs.min_edge_mm(),
            min_height_mm: 1.5 * self.thickness_mm + self.web_mm,
        }
    }

    /// How far in from a long edge the riser's middle plane stands, once folded.
    fn riser_in_mm(&self) -> f64 {
        self.thickness_mm / 2.0 + self.foot_mm
    }

    /// Smallest distance from a long edge to a hole: past the lock slot, plus a web.
    /// Products then also stand clear of the riser.
    fn hole_inset_mm(&self) -> f64 {
        self.riser_in_mm() + self.slot_width_mm / 2.0 + self.web_mm
    }

    /// The whole flat piece: the deck with its holes and lock slots, and both legs.
    fn piece(
        &self,
        (deck, holes, tabs): (Bounds, &[Area], &[(f64, f64)]),
    ) -> Result<BuiltPiece, GenerateError> {
        let t = self.thickness_mm;
        let across = (deck.min.x, deck.max.x);
        let deck_board = rect((
            across,
            (
                Side::Back.crease_y((deck, t)),
                Side::Front.crease_y((deck, t)),
            ),
        ));
        let slots: Vec<Polygon> = Side::BOTH
            .iter()
            .flat_map(|&side| self.slots((side, deck, tabs)))
            .collect();
        let cuts = Area::union_all(holes)?.union(&Area::from_polygons(&slots)?)?;
        let deck_area = Area::from_polygon(&deck_board)?.difference(&cuts)?;
        // Checked before the whole piece: holes that cut the deck in two also split the
        // piece, and the deck is the part to name.
        one_region((DECK_NAME, deck_area.clone()))?;

        let mut piece = FoldingPiece::new((PIECE_NAME, deck_area));
        for side in Side::BOTH {
            let leg = self.leg((side, deck, tabs))?;
            let mut parent = 0;
            for (stage, (area, crease_y)) in (0..).zip(leg.panels.into_iter().zip(leg.creases_y)) {
                let joint = Joint {
                    parent,
                    axis: along_x((crease_y, across)),
                    deg: LEG_FOLD_DEG,
                    direction: FoldDirection::Mountain,
                    offset_mm: 0.0,
                    stage,
                };
                parent = piece.fold((area, joint));
            }
        }
        piece.build()
    }

    /// The deck's lock slots along one long edge: one per tab, the tab plus the
    /// tolerance long and a slot wide, centred where the riser stands once folded.
    fn slots(&self, (side, deck, tabs): (Side, Bounds, &[(f64, f64)])) -> Vec<Polygon> {
        let edge = match side {
            Side::Back => deck.min.y,
            Side::Front => deck.max.y,
        };
        let riser_y = edge - side.outward() * self.riser_in_mm();
        let half_width = self.slot_width_mm / 2.0;
        let half_tolerance = self.tabs.tolerance_mm / 2.0;
        tabs.iter()
            .map(|&(start, end)| {
                rect((
                    (
                        deck.min.x + start - half_tolerance,
                        deck.min.x + end + half_tolerance,
                    ),
                    (riser_y - half_width, riser_y + half_width),
                ))
            })
            .collect()
    }

    /// One leg laid flat beyond a long edge of the deck: wall, foot, then the riser with
    /// its lock tabs, and the crease before each.
    fn leg(&self, (side, deck, tabs): (Side, Bounds, &[(f64, f64)])) -> Result<Leg, GenerateError> {
        let out = side.outward();
        let first = side.crease_y((deck, self.thickness_mm));
        let wall_end = first + out * self.wall_mm;
        let foot_end = wall_end + out * self.foot_mm;
        let riser_end = foot_end + out * self.riser_mm;
        let tab_end = riser_end + out * self.thickness_mm;
        let across = (deck.min.x, deck.max.x);
        let mut riser = vec![rect((across, (foot_end, riser_end)))];
        riser.extend(tabs.iter().map(|&(start, end)| {
            rect(((deck.min.x + start, deck.min.x + end), (riser_end, tab_end)))
        }));
        Ok(Leg {
            creases_y: [first, wall_end, foot_end],
            panels: [
                Area::from_polygon(&rect((across, (first, wall_end))))?,
                Area::from_polygon(&rect((across, (wall_end, foot_end))))?,
                Area::from_polygons(&riser)?,
            ],
        })
    }
}

/// One leg, flat, in the deck's frame.
struct Leg {
    /// The crease before each panel, as a y: deck to wall, wall to foot, foot to riser.
    creases_y: [f64; LEG_PANELS],
    /// Wall, foot, riser (with its tabs).
    panels: [Area; LEG_PANELS],
}
