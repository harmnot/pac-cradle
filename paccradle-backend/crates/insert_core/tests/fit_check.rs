//! M1-A8: the 2D fit check (CLAUDE.md §7.3). Every product copy gets a colour by the room
//! round it, seen from above: green at the board's recommended clearance, yellow when
//! tighter, red when it runs into board, another product, a cradle leg or the insert's
//! edge.
// The helpers below are test code too; clippy.toml only exempts `#[test]` functions.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use insert_core::{
    Board, BoxSize, BoxSpec, BoxStyle, Clash, Design, FitColor, InsertSpec, Material, Packaging,
    Placement, Point, ProductFit, Stance,
};

/// The 0.01 mm accuracy bar (M1-A2): measured clearances land within it.
const ACCURACY_MM: f64 = 0.01;

fn generate(design: &Design) -> Packaging {
    match design.generate() {
        Ok(packaging) => packaging,
        Err(error) => panic!("generate: {error}"),
    }
}

fn fits(design: &Design) -> Vec<ProductFit> {
    generate(design).fits(design).unwrap()
}

fn placed((id, x, y): (&str, f64, f64)) -> Placement {
    Placement {
        product_id: id.to_owned(),
        center_mm: Point::new(x, y),
        rotation_deg: 0.0,
    }
}

/// The sample card's lip gloss and compact, laid out by the generator on a growing
/// paperboard card (1 mm recommended clearance).
fn card() -> Design {
    let mut design = common::card_design();
    if let InsertSpec::CutoutCard(spec) = &mut design.insert {
        spec.placements.clear();
    }
    design.outer_box = None;
    design
}

/// One perfume bottle (52 × 38 mm) on a chipboard cradle, in a fixed mailer: the deck is
/// 157 × 97 mm, and the legs reach 6.9 mm in under each long edge.
fn cradle(center_y: f64) -> Design {
    let mut design = common::cradle_design();
    design.outer_box = Some(BoxSpec {
        style: BoxStyle::Mailer,
        material: Material::starting_values(Board::EFlute),
        inside_mm: Some(BoxSize {
            length_mm: 160.0,
            width_mm: 100.0,
            height_mm: 120.0,
        }),
    });
    if let InsertSpec::CradlePlatform(spec) = &mut design.insert {
        spec.placements = vec![placed(("perfume", 78.5, center_y))];
    }
    design
}

fn assert_green_at(fits: &[ProductFit], clearance_mm: f64) {
    for fit in fits {
        assert_eq!(
            (fit.color, fit.clashes.as_slice()),
            (FitColor::Green, &[][..]),
            "{fit:?}"
        );
        assert!(
            (fit.clearance_mm - clearance_mm).abs() < ACCURACY_MM,
            "{fit:?}"
        );
    }
}

#[test]
fn generated_designs_fit_at_the_recommended_clearance() {
    let fixture: Design =
        serde_json::from_str(include_str!("fixtures/cradle_mailer.json")).unwrap();
    // Paperboard and chipboard recommend 1 mm, E-flute 1.5 mm.
    for (design, clearance) in [
        (common::cradle_design(), 1.0),
        (fixture, 1.0),
        (card(), 1.0),
        (common::grid_design(), 1.5),
    ] {
        let found = fits(&design);
        assert_eq!(found.len(), generate(&design).insert.placements.len());
        assert_green_at(&found, clearance);
        assert!(found.iter().all(|fit| fit.target_mm == clearance));
    }

    // Framed grid strips stand round the outside cells too.
    let mut framed = common::grid_design();
    if let InsertSpec::PartitionGrid(spec) = &mut framed.insert {
        spec.outer_frame = true;
    }
    assert_green_at(&fits(&framed), 1.5);

    // Fits come in placement order.
    let order: Vec<usize> = fits(&common::grid_design())
        .iter()
        .map(|fit| fit.placement)
        .collect();
    assert_eq!(order, [0, 1, 2, 3, 4, 5]);
}

#[test]
fn less_clearance_than_recommended_is_yellow_and_none_is_red() {
    let mut snug = card();
    snug.material.clearance_mm = 0.5;
    for fit in fits(&snug) {
        assert_eq!(fit.color, FitColor::Yellow, "{fit:?}");
        assert!((fit.clearance_mm - 0.5).abs() < ACCURACY_MM && fit.target_mm == 1.0);
    }

    let mut touching = card();
    touching.material.clearance_mm = 0.0;
    for fit in fits(&touching) {
        assert_eq!(
            (fit.color, fit.clashes.as_slice()),
            (FitColor::Red, &[Clash::Board][..])
        );
        assert!(fit.clearance_mm < ACCURACY_MM);
    }
}

#[test]
fn overlapping_products_are_red_and_name_each_other() {
    // Two 20 mm squares 10 mm apart: their holes merge into one.
    let mut design = card();
    design.products = vec![{
        let mut square = common::product("square", common::rectangle(20.0, 20.0));
        square.quantity = 2;
        square
    }];
    if let InsertSpec::CutoutCard(spec) = &mut design.insert {
        spec.finger_notches = None;
        spec.placements = vec![
            placed(("square", 30.0, 30.0)),
            placed(("square", 40.0, 30.0)),
        ];
    }
    let found = fits(&design);
    assert_eq!(found[0].clashes, [Clash::Product { placement: 1 }]);
    assert_eq!(found[1].clashes, [Clash::Product { placement: 0 }]);
    for fit in &found {
        assert_eq!(fit.color, FitColor::Red);
        // The merged hole still clears the board.
        assert!((fit.clearance_mm - 1.0).abs() < ACCURACY_MM, "{fit:?}");
    }
}

#[test]
fn a_product_off_the_card_is_red() {
    // The sample card pins the gloss so it hangs off the card's left edge.
    let mut design = common::card_design();
    design
        .outer_box
        .as_mut()
        .unwrap()
        .inside_mm
        .as_mut()
        .unwrap()
        .height_mm = 100.0;
    let found = fits(&design);
    assert_eq!(found[0].color, FitColor::Red);
    assert_eq!(found[0].clashes, [Clash::OffInsert]);
    assert_eq!(found[0].clearance_mm, 0.0);
    // The compact, laid out by the generator, still fits.
    assert_green_at(&found[1..], 1.0);
}

#[test]
fn cradle_legs_count_as_board_under_the_deck() {
    assert_green_at(&fits(&cradle(48.5)), 1.0);

    // 0.5 mm from the leg's strip: fits, but tight.
    let near = fits(&cradle(6.9 + 0.5 + 19.0));
    assert_eq!(near[0].color, FitColor::Yellow);
    assert!(
        (near[0].clearance_mm - 0.5).abs() < ACCURACY_MM,
        "{:?}",
        near[0]
    );

    let over = fits(&cradle(20.0));
    assert_eq!(
        (over[0].color, over[0].clashes.as_slice()),
        (FitColor::Red, &[Clash::Leg][..])
    );
}

#[test]
fn a_product_on_solid_board_or_in_a_strip_is_red() {
    // The card with its holes taken out: the products stand on board.
    let design = card();
    let mut no_holes = generate(&design);
    no_holes.insert.pieces[0].panels[0].holes.clear();
    for fit in no_holes.fits(&design).unwrap() {
        assert_eq!(
            (fit.color, fit.clashes.as_slice()),
            (FitColor::Red, &[Clash::Board][..])
        );
        assert_eq!(fit.clearance_mm, 0.0);
    }

    // A grid strip moved 10 mm into the first row of cells.
    let design = common::grid_design();
    let mut moved = generate(&design);
    let strip = moved
        .insert
        .mounts
        .iter()
        .position(|mount| mount.stance == Stance::UprightAlongX)
        .unwrap();
    moved.insert.mounts[strip].at_mm[1] -= 10.0;
    let found = moved.fits(&design).unwrap();
    let red: Vec<usize> = found
        .iter()
        .filter(|fit| fit.color == FitColor::Red)
        .map(|fit| fit.placement)
        .collect();
    // Row by row from the top-left: the first row is placements 0 to 2.
    assert_eq!(red, [0, 1, 2]);
    assert!(found[..3].iter().all(|fit| fit.clashes == [Clash::Board]));
}

#[test]
fn fits_serialize_with_colour_and_clashes() {
    let fit = ProductFit {
        placement: 0,
        color: FitColor::Red,
        clearance_mm: 0.0,
        target_mm: 1.5,
        clashes: vec![Clash::Product { placement: 1 }, Clash::OffInsert],
    };
    let value = serde_json::to_value(&fit).unwrap();
    assert_eq!(value["color"], "red");
    assert_eq!(
        value["clashes"],
        serde_json::json!([{ "kind": "product", "placement": 1 }, { "kind": "off_insert" }])
    );
    let back: ProductFit = serde_json::from_value(value).unwrap();
    assert_eq!(back, fit);
}
