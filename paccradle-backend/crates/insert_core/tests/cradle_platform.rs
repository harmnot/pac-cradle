//! M1-A5: the cradle platform. The main check folds the generated piece along its hinges
//! and looks at the result, not at the generator's own numbers. It checks that:
//! - no panel runs into another;
//! - the feet rest on the floor;
//! - the lock tabs pass through their slots and end flush with the deck top;
//! - no other board pokes through the deck;
//! - the products stand clear of the legs.
//!
//! Also: holes, deck sizes, boxes of fixed size, tab counts and the errors.
// The helpers below are test code too; clippy.toml only exempts `#[test]` functions.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::fold::{V3, fold, hinge_problems};
use common::measure::{distance_to_outline, is_inside, rectangle_at};
use common::placed_outline;
use common::solid::{collisions, loose_hinges, solids};
use insert_core::{
    Align, Area, Board, BoxSize, BoxSpec, BoxStyle, CradlePlatformSpec, Design, FoldDirection,
    GenerateError, Insert, InsertSpec, Material, Placement, Point, Polygon, Product, Transform,
};

/// Chipboard starting values (CLAUDE.md §6.4).
const THICKNESS_MM: f64 = 1.5;
const CLEARANCE_MM: f64 = 1.0;
const SLOT_WIDTH_MM: f64 = THICKNESS_MM + 0.3;
/// Chipboard's narrowest web: 3× 1.5 mm.
const WEB_MM: f64 = 4.5;
/// From a long edge to the riser's middle plane: half a board, then the foot (half a
/// slot plus a web).
const RISER_IN_MM: f64 = THICKNESS_MM / 2.0 + SLOT_WIDTH_MM / 2.0 + WEB_MM;
/// From a long edge to the nearest hole the legs allow: past the slot, plus a web.
const LEG_INSET_MM: f64 = RISER_IN_MM + SLOT_WIDTH_MM / 2.0 + WEB_MM;
const DECK_HEIGHT_MM: f64 = 40.0;
/// The M1-A2 accuracy bar, applied to the holes.
const ACCURACY_MM: f64 = 0.01;
/// Slack for positions read back from folded panels: the polygon library snaps to 0.1 µm.
const EPS_MM: f64 = 1e-3;

fn product((id, contour_mm): (&str, Polygon), quantity: u32) -> Product {
    Product {
        id: id.to_owned(),
        contour_mm,
        height_mm: 110.0,
        quantity,
    }
}

/// A perfume bottle with a 52 × 38 mm footprint.
fn bottles(quantity: u32) -> Product {
    product(
        ("perfume", rectangle_at(Point::ORIGIN, (52.0, 38.0))),
        quantity,
    )
}

fn cradle(margin_mm: f64) -> CradlePlatformSpec {
    CradlePlatformSpec {
        deck_height_mm: DECK_HEIGHT_MM,
        margin_mm,
        align: Align::default(),
        placements: Vec::new(),
    }
}

/// A chipboard cradle with no box.
fn design(products: Vec<Product>, cradle: CradlePlatformSpec) -> Design {
    Design {
        material: Material::starting_values(Board::Chipboard),
        products,
        insert: InsertSpec::CradlePlatform(cradle),
        outer_box: None,
    }
}

/// A mailer box of E-flute (1.5 mm clearance) with a fixed inside size.
fn fixed_box((length_mm, width_mm): (f64, f64)) -> Option<BoxSpec> {
    Some(BoxSpec {
        style: BoxStyle::Mailer,
        material: Material::starting_values(Board::EFlute),
        inside_mm: Some(BoxSize {
            length_mm,
            width_mm,
            height_mm: 120.0,
        }),
    })
}

fn generate(design: &Design) -> Insert {
    match design.generate_insert() {
        Ok(insert) => insert,
        Err(error) => panic!("generate: {error}"),
    }
}

fn to_2d(ring: &[V3]) -> Polygon {
    Polygon(ring.iter().map(|p| Point::new(p[0], p[1])).collect())
}

/// What's wrong with the folded cradle: an empty list when it folds up right.
fn fold_problems((insert, design): (&Insert, &Design)) -> Vec<String> {
    let t = design.material.thickness_mm;
    let piece = &insert.pieces[0];
    let mut problems = hinge_problems(piece);
    let folded = match fold((piece, &insert.mounts[0], t)) {
        Ok(folded) => folded,
        Err(error) => return vec![error],
    };
    match solids((&folded, t)) {
        Ok(solids) => {
            problems.extend(
                collisions((&solids, &piece.hinges))
                    .into_iter()
                    .map(|(i, j)| format!("panels {i} and {j} run into each other")),
            );
            problems.extend(
                loose_hinges((&solids, &piece.hinges))
                    .into_iter()
                    .map(|(parent, child)| {
                        format!("panel {child} comes loose from panel {parent}")
                    }),
            );
        }
        Err(error) => problems.push(error),
    }
    let (length, width, height) = (insert.length_mm, insert.width_mm, insert.height_mm);
    let deck_holes: Vec<Polygon> = folded[0].holes.iter().map(|ring| to_2d(ring)).collect();
    let deck_outline = to_2d(&folded[0].outline);
    // Where a wall meets the deck at its crease, on the deck's middle plane.
    let on_deck_edge = |p: V3| {
        (p[2] - (height - t / 2.0)).abs() <= EPS_MM
            && distance_to_outline(Point::new(p[0], p[1]), &deck_outline) <= EPS_MM
    };
    let board: Vec<V3> = folded[1..]
        .iter()
        .flat_map(|panel| panel.outline.iter().copied())
        .collect();

    let lowest = board.iter().map(|p| p[2]).fold(f64::INFINITY, f64::min);
    let highest = board.iter().map(|p| p[2]).fold(f64::NEG_INFINITY, f64::max);
    if (lowest - t / 2.0).abs() > EPS_MM {
        problems.push(format!(
            "the feet don't rest on the floor: lowest at {lowest}"
        ));
    }
    if (highest - height).abs() > EPS_MM {
        problems.push(format!(
            "the tabs don't end flush with the deck: highest at {highest}"
        ));
    }
    let mut tab_tops = Vec::new();
    for &p in &board {
        let inside_x = (-EPS_MM..=length + EPS_MM).contains(&p[0]);
        let inside_y = (t / 2.0 - EPS_MM..=width - t / 2.0 + EPS_MM).contains(&p[1]);
        if !inside_x || !inside_y {
            problems.push(format!("board at {p:?} sticks out of the footprint"));
        }
        if p[2] <= height - t + EPS_MM || on_deck_edge(p) {
            continue;
        }
        // Board above the deck's underside must pass through a slot, its whole thickness.
        let through = [-t / 2.0, t / 2.0].iter().all(|dy| {
            let face = Point::new(p[0], p[1] + dy);
            deck_holes.iter().any(|hole| is_inside(face, hole))
        });
        match through {
            true => tab_tops.push(Point::new(p[0], p[1])),
            false => problems.push(format!("board at {p:?} goes through the deck, not a slot")),
        }
    }

    let products_in = |hole: &Polygon| {
        insert
            .placements
            .iter()
            .any(|placement| is_inside(placement.center_mm, hole))
    };
    for slot in deck_holes.iter().filter(|hole| !products_in(hole)) {
        if !tab_tops.iter().any(|&top| is_inside(top, slot)) {
            problems.push("a lock slot gets no tab".to_owned());
        }
    }

    for placement in &insert.placements {
        let product = design
            .products
            .iter()
            .find(|product| product.id == placement.product_id)
            .unwrap();
        let outline = placed_outline(product, placement).bounds().unwrap();
        for (index, panel) in folded.iter().enumerate().skip(1) {
            let ys = panel.outline.iter().map(|p| p[1]);
            let near = ys.clone().fold(f64::INFINITY, f64::min) - t / 2.0;
            let far = ys.fold(f64::NEG_INFINITY, f64::max) + t / 2.0;
            if outline.min.y < far && outline.max.y > near {
                problems.push(format!("`{}` runs into leg panel {index}", product.id));
            }
        }
    }
    problems
}

/// A named way to break a generated insert on purpose.
type Break = (&'static str, fn(&mut Insert));

fn assert_folds_up(design: &Design) -> Insert {
    let insert = generate(design);
    let problems = fold_problems((&insert, design));
    assert!(problems.is_empty(), "{problems:#?}");
    insert
}

/// The deck's holes that hold products (not the lock slots), as areas.
fn product_holes(insert: &Insert) -> Vec<Area> {
    let at = insert.mounts[0].at_mm;
    let to_insert = Transform::shift(Point::new(at[0], at[1]));
    insert.pieces[0].panels[0]
        .holes
        .iter()
        .map(|hole| hole.transformed(to_insert))
        .filter(|hole| {
            insert
                .placements
                .iter()
                .any(|placement| is_inside(placement.center_mm, hole))
        })
        .map(|hole| Area::from_polygon(&hole).unwrap())
        .collect()
}

fn lock_slot_count(insert: &Insert) -> usize {
    insert.pieces[0].panels[0].holes.len() - product_holes(insert).len()
}

#[test]
fn one_piece_with_a_deck_and_two_folding_legs() {
    let insert = generate(&common::cradle_design());

    assert_eq!(insert.pieces.len(), 1);
    let piece = &insert.pieces[0];
    assert_eq!((piece.name.as_str(), piece.quantity), ("cradle", 1));
    assert_eq!(piece.panels.len(), 7);
    let tree: Vec<(usize, usize, u32)> = piece
        .hinges
        .iter()
        .map(|hinge| (hinge.parent, hinge.child, hinge.stage))
        .collect();
    assert_eq!(
        tree,
        [
            (0, 1, 0),
            (1, 2, 1),
            (2, 3, 2),
            (0, 4, 0),
            (4, 5, 1),
            (5, 6, 2)
        ]
    );
    for hinge in &piece.hinges {
        assert_eq!(hinge.fold_deg, 90.0);
        assert_eq!(hinge.direction, FoldDirection::Mountain);
    }
    assert_eq!(piece.creases.len(), 6);
    assert_eq!(insert.height_mm, DECK_HEIGHT_MM);
    assert_eq!(insert.mounts.len(), 1);
    assert!((insert.mounts[0].at_mm[2] - (DECK_HEIGHT_MM - THICKNESS_MM)).abs() < 1e-9);
}

#[test]
fn legs_fold_under_the_deck_and_lock_into_its_slots() {
    let jars = product(("jar", common::circle(45.0)), 3);
    let mut e_flute = design(vec![jars], cradle(10.0));
    e_flute.material = Material::starting_values(Board::EFlute);
    let mut placed = design(vec![bottles(2)], cradle(12.0));
    placed.outer_box = fixed_box((180.0, 90.0));

    for design in [common::cradle_design(), e_flute, placed] {
        assert_folds_up(&design);
    }
}

#[test]
fn documented_fixture_folds_up() {
    let fixture = include_str!("fixtures/cradle_mailer.json");
    let design: Design = serde_json::from_str(fixture).unwrap();
    assert_folds_up(&design);
}

#[test]
fn fold_check_catches_broken_cradles() {
    let design = common::cradle_design();
    let insert = generate(&design);
    let breaks: [Break; 4] = [
        ("slots 0.5 mm off", |insert| {
            for hole in &mut insert.pieces[0].panels[0].holes {
                *hole = hole.transformed(Transform::shift(Point::new(0.0, 0.5)));
            }
        }),
        ("a foot folded to 80°", |insert| {
            insert.pieces[0].hinges[1].fold_deg = 80.0;
        }),
        ("a wall folded up", |insert| {
            insert.pieces[0].hinges[0].direction = FoldDirection::Valley;
        }),
        ("a crease off its edge", |insert| {
            insert.pieces[0].hinges[2].axis.from.y += 1.0;
        }),
    ];
    for (name, break_it) in breaks {
        let mut broken = insert.clone();
        break_it(&mut broken);
        assert!(
            !fold_problems((&broken, &design)).is_empty(),
            "{name} wasn't caught"
        );
    }
}

#[test]
fn hole_is_the_outline_plus_clearance() {
    let jar = product(("jar", common::circle(45.0)), 1);
    let design = design(vec![jar.clone(), bottles(1)], cradle(10.0));
    let insert = generate(&design);
    let holes = product_holes(&insert);
    assert_eq!(holes.len(), 2);

    for placement in &insert.placements {
        let product = design
            .products
            .iter()
            .find(|product| product.id == placement.product_id)
            .unwrap();
        let outline = placed_outline(product, placement);
        let hole = holes
            .iter()
            .find(|hole| !hole.intersection(&outline).unwrap().is_empty())
            .unwrap();
        let inside = outline.offset(CLEARANCE_MM - ACCURACY_MM).unwrap();
        let outside = outline.offset(CLEARANCE_MM + ACCURACY_MM).unwrap();
        assert!(
            inside.difference(hole).unwrap().area_mm2() < 1e-6,
            "too tight"
        );
        assert!(
            hole.difference(&outside).unwrap().area_mm2() < 1e-6,
            "too loose"
        );
    }
}

#[test]
fn deck_grows_round_the_holes_leaving_room_for_the_legs() {
    // The hole is 54 × 40 mm (52 × 38 plus 1 mm each side).
    let small_margin = generate(&design(vec![bottles(1)], cradle(5.0)));
    assert!((small_margin.length_mm - (54.0 + 2.0 * 5.0)).abs() < EPS_MM);
    assert!((small_margin.width_mm - (40.0 + 2.0 * LEG_INSET_MM)).abs() < EPS_MM);

    let big_margin = generate(&design(vec![bottles(1)], cradle(15.0)));
    assert!((big_margin.length_mm - (54.0 + 30.0)).abs() < EPS_MM);
    assert!((big_margin.width_mm - (40.0 + 30.0)).abs() < EPS_MM);
}

#[test]
fn fixed_box_sets_the_deck_size_and_align_moves_the_products() {
    let mut centred = design(vec![bottles(1)], cradle(10.0));
    centred.outer_box = fixed_box((160.0, 100.0));
    let insert = generate(&centred);
    // Inside size minus the box's 1.5 mm clearance each side.
    assert_eq!((insert.length_mm, insert.width_mm), (157.0, 97.0));
    let center = insert.placements[0].center_mm;
    assert!(center.distance_to(Point::new(78.5, 48.5)) < EPS_MM);

    let mut left = centred.clone();
    if let InsertSpec::CradlePlatform(spec) = &mut left.insert {
        spec.align = Align::TopLeft;
    }
    let insert = generate(&left);
    let hole = product_holes(&insert)[0].bounds().unwrap();
    assert!((hole.min.x - 10.0).abs() < EPS_MM);
    assert!((hole.min.y - LEG_INSET_MM).abs() < EPS_MM);
    assert!(fold_problems((&insert, &left)).is_empty());
}

#[test]
fn products_too_big_for_the_box_are_reported() {
    let mut too_small = design(vec![bottles(3)], cradle(10.0));
    too_small.outer_box = fixed_box((120.0, 70.0));
    assert!(matches!(
        too_small.generate_insert(),
        Err(GenerateError::DoesNotFit { .. })
    ));
}

#[test]
fn longer_decks_get_more_lock_tabs() {
    // 2 tabs per leg up to about 130 mm; then about one per 60 mm.
    let one = generate(&design(vec![bottles(1)], cradle(10.0)));
    assert_eq!(lock_slot_count(&one), 4);
    // A long, narrow box keeps the five bottles in one row.
    let mut long = design(vec![bottles(5)], cradle(10.0));
    long.outer_box = fixed_box((312.0, 70.0));
    let five = assert_folds_up(&long);
    assert_eq!(five.length_mm, 309.0);
    // floor((309 − 2 × 4.5) / 60) = 5 tabs per leg.
    assert_eq!(lock_slot_count(&five), 10);
}

#[test]
fn deck_too_low_or_too_short_is_refused() {
    let too_small = Err(GenerateError::DeckTooSmall {
        min_length_mm: 3.0 * WEB_MM + 0.3 + 5.0,
        min_height_mm: 1.5 * THICKNESS_MM + WEB_MM,
    });
    let mut low = cradle(10.0);
    low.deck_height_mm = 6.0;
    assert_eq!(design(vec![bottles(1)], low).generate_insert(), too_small);

    let pin = product(("pin", rectangle_at(Point::ORIGIN, (8.0, 8.0))), 1);
    assert_eq!(design(vec![pin], cradle(2.0)).generate_insert(), too_small);
}

#[test]
fn hole_across_the_whole_deck_is_refused() {
    let rod = product(("rod", rectangle_at(Point::ORIGIN, (200.0, 10.0))), 1);
    let mut spec = cradle(10.0);
    spec.placements = vec![Placement {
        product_id: "rod".to_owned(),
        center_mm: Point::new(78.5, 48.5),
        rotation_deg: 0.0,
    }];
    let mut design = design(vec![rod], spec);
    design.outer_box = fixed_box((160.0, 100.0));
    assert_eq!(
        design.generate_insert(),
        Err(GenerateError::FallsApart {
            piece: "deck",
            parts: 2
        })
    );
}

#[test]
fn returned_placements_rebuild_the_same_insert() {
    let jars = product(("jar", common::circle(45.0)), 3);
    let first_design = design(vec![jars, bottles(2)], cradle(10.0));
    let first = generate(&first_design);

    let mut again = first_design.clone();
    if let InsertSpec::CradlePlatform(spec) = &mut again.insert {
        spec.placements = first.placements.clone();
    }
    assert_eq!(generate(&again), first);
}
