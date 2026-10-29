//! M1-A3: the cutout card. Holes are the outline plus clearance (within 0.01 mm), the
//! card is the box size minus clearance or grows around the holes, and finger notches
//! sit on a long side.
// The helpers below are test code too; clippy.toml only exempts `#[test]` functions.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::measure::{distance_to_outline, rectangle_at};
use common::placed_outline;
use insert_core::{
    Align, Area, Board, BoxSize, BoxSpec, BoxStyle, CURVE_TOLERANCE_MM, CutoutCardSpec, Design,
    DesignError, FINGER_NOTCH_RADIUS_MM, FingerNotches, GenerateError, InsertSpec, Material,
    NotchSide, Path, Placement, Point, Polygon, Product, Transform,
};

/// The M1-A2 accuracy bar, applied to the holes.
const ACCURACY_MM: f64 = 0.01;
/// Paperboard's starting clearance.
const CLEARANCE_MM: f64 = 1.0;
/// Paperboard's narrowest web between holes: 3 mm (3× 0.5 mm is less).
const WEB_MM: f64 = 3.0;

fn product(id: &str, contour_mm: Polygon) -> Product {
    Product {
        id: id.to_owned(),
        contour_mm,
        height_mm: 40.0,
        quantity: 1,
    }
}

/// A round product `diameter_mm` across, its outline inside its bounding box from the origin.
fn round(id: &str, diameter_mm: f64) -> Product {
    let radius = diameter_mm / 2.0;
    let outline = Path::circle(Point::new(radius, radius), radius).flatten(CURVE_TOLERANCE_MM);
    product(id, Polygon(outline))
}

fn boxy(id: &str, size_mm: (f64, f64)) -> Product {
    product(id, rectangle_at(Point::ORIGIN, size_mm))
}

fn card(margin_mm: f64, finger_notches: bool) -> CutoutCardSpec {
    CutoutCardSpec {
        margin_mm,
        finger_notches: finger_notches.then(FingerNotches::default),
        align: Align::default(),
        placements: Vec::new(),
    }
}

/// A card whose holes get notches set by `notches`.
fn notched_card(notches: FingerNotches) -> CutoutCardSpec {
    CutoutCardSpec {
        finger_notches: Some(notches),
        ..card(10.0, false)
    }
}

/// The part of a single-product card's hole that the notches add: the hole minus the
/// product's outline grown by the clearance.
fn notch_area((product, insert): (&Product, &insert_core::Insert)) -> Area {
    let outline = placed_outline(product, &insert.placements[0]);
    let plain = outline.offset(CLEARANCE_MM + ACCURACY_MM).unwrap();
    holes(insert)[0].difference(&plain).unwrap()
}

/// A paperboard card design with no box.
fn design(products: Vec<Product>, card: CutoutCardSpec) -> Design {
    Design {
        material: Material::starting_values(Board::Paperboard),
        products,
        insert: InsertSpec::CutoutCard(card),
        outer_box: None,
    }
}

/// A shipping box of B-flute (1.75 mm clearance) with a fixed inside size.
fn fixed_box(length_mm: f64, width_mm: f64) -> Option<BoxSpec> {
    Some(BoxSpec {
        style: BoxStyle::Shipping,
        material: Material::starting_values(Board::BFlute),
        inside_mm: Some(BoxSize {
            length_mm,
            width_mm,
            height_mm: 60.0,
        }),
    })
}

fn placement(product_id: &str, center: [f64; 2]) -> Placement {
    Placement {
        product_id: product_id.to_owned(),
        center_mm: Point::from(center),
        rotation_deg: 0.0,
    }
}

/// The card's holes as areas.
fn holes(insert: &insert_core::Insert) -> Vec<Area> {
    insert.pieces[0].panels[0]
        .holes
        .iter()
        .map(|hole| Area::from_polygon(hole).unwrap())
        .collect()
}

/// Asserts the hole is the product's outline plus the clearance, within the accuracy bar.
fn assert_hole_fits((product, placement): (&Product, &Placement), hole: &Area) {
    let outline = placed_outline(product, placement);
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

fn generate(design: &Design) -> insert_core::Insert {
    match design.generate_insert() {
        Ok(insert) => insert,
        Err(error) => panic!("generate: {error}"),
    }
}

#[test]
fn round_product_gets_a_round_hole_on_a_card_grown_around_it() {
    let jar = round("jar", 60.0);
    let insert = generate(&design(vec![jar.clone()], card(10.0, false)));

    let [piece] = insert.pieces.as_slice() else {
        panic!("expected one piece");
    };
    assert_eq!(piece.name, "card");
    assert_eq!(piece.holes.len(), 1);
    assert!(piece.hinges.is_empty());
    assert_eq!(
        piece.panels[0].outline.points().len(),
        4,
        "a plain rectangle"
    );
    // 60 mm + 1 mm clearance each side, + 10 mm margin each side.
    assert!((insert.length_mm - 82.0).abs() < ACCURACY_MM);
    assert!((insert.width_mm - 82.0).abs() < ACCURACY_MM);
    assert_eq!(insert.height_mm, 0.5, "a flat card is one board thick");
    assert_eq!(
        insert.mounts,
        vec![insert_core::Mount {
            piece: 0,
            stance: insert_core::Stance::Flat,
            at_mm: [0.0, 0.0, 0.0],
        }],
        "the card lies flat on the box floor"
    );
    let [placed] = insert.placements.as_slice() else {
        panic!("expected one placement");
    };
    assert!(placed.center_mm.distance_to(Point::new(41.0, 41.0)) < ACCURACY_MM);
    assert_hole_fits((&jar, placed), &holes(&insert)[0]);
}

#[test]
fn hole_is_the_outline_plus_clearance() {
    let gloss = boxy("gloss", (18.0, 90.0));
    let insert = generate(&design(vec![gloss.clone()], card(10.0, false)));

    assert_hole_fits((&gloss, &insert.placements[0]), &holes(&insert)[0]);
    let exact = 18.0 * 90.0 + 2.0 * (18.0 + 90.0) + std::f64::consts::PI;
    assert!((holes(&insert)[0].area_mm2() - exact).abs() < 0.05);
}

#[test]
fn products_are_laid_long_side_along_x() {
    // The outline stands upright (18 wide, 90 tall); the card lies it down.
    let gloss = boxy("gloss", (18.0, 90.0));
    let insert = generate(&design(vec![gloss], card(10.0, false)));
    assert!((insert.length_mm - 112.0).abs() < ACCURACY_MM);
    assert!((insert.width_mm - 40.0).abs() < ACCURACY_MM);
    assert_eq!(insert.placements[0].rotation_deg, 90.0);
}

#[test]
fn product_photographed_at_an_angle_is_laid_straight() {
    let straight = rectangle_at(Point::ORIGIN, (50.0, 30.0));
    let photographed = straight.transformed(Transform {
        turn_deg: 30.0,
        shift: Point::new(20.0, 0.0),
    });
    let insert = generate(&design(
        vec![product("case", photographed)],
        card(10.0, false),
    ));
    assert!(
        (insert.length_mm - 72.0).abs() < ACCURACY_MM,
        "{}",
        insert.length_mm
    );
    assert!(
        (insert.width_mm - 52.0).abs() < ACCURACY_MM,
        "{}",
        insert.width_mm
    );
    assert!((insert.placements[0].rotation_deg + 30.0).abs() < 1e-9);
}

#[test]
fn finger_notch_sits_halfway_along_a_long_side() {
    let gloss = boxy("gloss", (18.0, 90.0));
    let plain = generate(&design(vec![gloss.clone()], card(10.0, false)));
    let notched = generate(&design(vec![gloss], card(10.0, true)));

    // The notch reaches past the hole on one long side only.
    assert!((notched.length_mm - plain.length_mm).abs() < ACCURACY_MM);
    let deeper = notched.width_mm - plain.width_mm;
    assert!(
        (deeper - FINGER_NOTCH_RADIUS_MM).abs() < ACCURACY_MM,
        "{deeper}"
    );
    let hole = &holes(&notched)[0];
    let bounds = hole.bounds().unwrap();
    assert!((bounds.center().x - notched.length_mm / 2.0).abs() < ACCURACY_MM);
    // About half a notch circle is added (its centre sits on the hole's edge).
    let added = hole.area_mm2() - holes(&plain)[0].area_mm2();
    let half_circle = std::f64::consts::PI * FINGER_NOTCH_RADIUS_MM.powi(2) / 2.0;
    assert!((added - half_circle).abs() < 2.0, "{added}");
}

#[test]
fn copies_are_laid_in_rows_with_board_between_them() {
    let mut jar = round("jar", 40.0);
    jar.quantity = 6;
    let margin = 10.0;
    let insert = generate(&design(vec![jar.clone()], card(margin, false)));

    let holes = holes(&insert);
    assert_eq!(holes.len(), 6);
    assert_eq!(insert.placements.len(), 6);
    for hole in &holes {
        let bounds = hole.bounds().unwrap();
        assert!(bounds.min.x >= margin - ACCURACY_MM && bounds.min.y >= margin - ACCURACY_MM);
        assert!(bounds.max.x <= insert.length_mm - margin + ACCURACY_MM);
        assert!(bounds.max.y <= insert.width_mm - margin + ACCURACY_MM);
    }
    for (index, hole) in holes.iter().enumerate() {
        for other in &holes[index + 1..] {
            let gap = ring_gap((hole, other));
            assert!(gap >= WEB_MM - ACCURACY_MM, "holes {gap} mm apart");
        }
    }
    // Every hole matches its placement, whatever order the pieces list them in.
    for placed in &insert.placements {
        let hole = holes
            .iter()
            .find(|hole| {
                hole.bounds()
                    .unwrap()
                    .center()
                    .distance_to(placed.center_mm)
                    < 0.1
            })
            .expect("a hole for every placement");
        assert_hole_fits((&jar, placed), hole);
    }
    let aspect = insert.length_mm.max(insert.width_mm) / insert.length_mm.min(insert.width_mm);
    assert!(aspect < 1.6, "a grown card stays roughly square: {aspect}");
}

/// Smallest distance between the outer rings of two separate areas.
fn ring_gap((a, b): (&Area, &Area)) -> f64 {
    let (a, b) = (&a.regions()[0].outer, &b.regions()[0].outer);
    let a_to_b = a
        .points()
        .iter()
        .map(|&point| distance_to_outline(point, b));
    let b_to_a = b
        .points()
        .iter()
        .map(|&point| distance_to_outline(point, a));
    a_to_b.chain(b_to_a).fold(f64::INFINITY, f64::min)
}

#[test]
fn fixed_box_sets_the_card_size_and_centres_the_products() {
    let mut design = design(vec![round("jar", 40.0)], card(10.0, false));
    design.outer_box = fixed_box(160.0, 120.0);
    let insert = generate(&design);

    // Inside size minus the box's 1.75 mm clearance on each side.
    assert!((insert.length_mm - 156.5).abs() < 1e-9);
    assert!((insert.width_mm - 116.5).abs() < 1e-9);
    assert!(
        insert.placements[0]
            .center_mm
            .distance_to(Point::new(78.25, 58.25))
            < ACCURACY_MM
    );
}

#[test]
fn products_too_big_for_the_box_are_reported() {
    let mut design = design(vec![round("plate", 80.0)], card(10.0, false));
    design.outer_box = fixed_box(60.0, 60.0);
    let Err(GenerateError::DoesNotFit {
        needed_length_mm,
        needed_width_mm,
        room_length_mm,
        room_width_mm,
    }) = design.generate_insert()
    else {
        panic!("expected DoesNotFit");
    };
    assert!((needed_length_mm - 102.0).abs() < ACCURACY_MM);
    assert!((needed_width_mm - 102.0).abs() < ACCURACY_MM);
    assert_eq!((room_length_mm, room_width_mm), (56.5, 56.5));
}

#[test]
fn long_product_turns_crosswise_to_fit_a_narrow_card() {
    // A 90 mm tube on a card 56.5 mm long and 116.5 mm wide only fits standing along y.
    let mut design = design(vec![boxy("tube", (90.0, 18.0))], card(5.0, false));
    design.outer_box = fixed_box(60.0, 120.0);
    let insert = generate(&design);
    let bounds = holes(&insert)[0].bounds().unwrap();
    assert!(bounds.width_mm() > bounds.length_mm());
}

#[test]
fn design_placements_stay_and_the_rest_fill_in() {
    let mut jar = round("jar", 40.0);
    jar.quantity = 2;
    let tube = boxy("tube", (50.0, 20.0));
    let mut spec = card(10.0, false);
    spec.placements = vec![placement("jar", [35.0, 35.0])];
    let mut design = design(vec![jar, tube], spec);
    design.outer_box = fixed_box(200.0, 120.0);
    let insert = generate(&design);

    assert_eq!(insert.placements.len(), 3);
    assert_eq!(insert.placements[0], placement("jar", [35.0, 35.0]));
    assert_eq!(insert.placements[1].product_id, "jar");
    assert_eq!(insert.placements[2].product_id, "tube");
    // The others go to the right of the placed jar, with board between.
    let jar_right_edge = 35.0 + 20.0 + CLEARANCE_MM;
    for hole in &holes(&insert)[1..] {
        assert!(hole.bounds().unwrap().min.x >= jar_right_edge + WEB_MM - ACCURACY_MM);
    }
}

#[test]
fn returned_placements_rebuild_the_same_insert() {
    let mut jar = round("jar", 40.0);
    jar.quantity = 3;
    let products = vec![jar, boxy("tube", (18.0, 90.0)), round("lid", 25.0)];
    let mut design = design(products, card(8.0, true));
    let first = generate(&design);

    design.insert = InsertSpec::CutoutCard(CutoutCardSpec {
        placements: first.placements.clone(),
        ..card(8.0, true)
    });
    let again = generate(&design);
    assert_eq!(again.placements, first.placements);
    assert!((again.length_mm - first.length_mm).abs() < 1e-9);
    assert!((again.width_mm - first.width_mm).abs() < 1e-9);
    assert_eq!(again.pieces[0].holes.len(), first.pieces[0].holes.len());
}

#[test]
fn overlapping_products_share_one_hole() {
    let mut jar = round("jar", 40.0);
    jar.quantity = 2;
    let mut spec = card(10.0, false);
    spec.placements = vec![
        placement("jar", [30.0, 30.0]),
        placement("jar", [50.0, 30.0]),
    ];
    let insert = generate(&design(vec![jar], spec));
    assert_eq!(insert.pieces[0].holes.len(), 1);
}

#[test]
fn hole_across_the_whole_card_is_refused() {
    let mut spec = card(5.0, false);
    spec.placements = vec![placement("rod", [50.0, 25.0])];
    let mut design = design(vec![boxy("rod", (150.0, 10.0))], spec);
    design.outer_box = fixed_box(103.5, 53.5);
    assert_eq!(
        design.generate_insert(),
        Err(GenerateError::FallsApart {
            piece: "card",
            parts: 2
        })
    );
}

#[test]
fn invalid_designs_are_refused_before_generating() {
    let mut design = design(vec![round("jar", 40.0)], card(10.0, false));
    design.material.thickness_mm = 0.0;
    assert!(matches!(
        design.generate_insert(),
        Err(GenerateError::Invalid(DesignError::LengthOutOfRange { .. }))
    ));
}

#[test]
fn insert_survives_a_json_round_trip() {
    let insert = generate(&design(vec![round("jar", 40.0)], card(10.0, true)));
    let json = serde_json::to_string(&insert).unwrap();
    let back: insert_core::Insert = serde_json::from_str(&json).unwrap();
    assert_eq!(back, insert);
}

#[test]
fn notch_can_sit_on_the_top_side() {
    let gloss = boxy("gloss", (18.0, 90.0));
    let top = FingerNotches {
        side: NotchSide::Top,
        ..FingerNotches::default()
    };
    let insert = generate(&design(vec![gloss.clone()], notched_card(top)));
    let notch = notch_area((&gloss, &insert)).bounds().unwrap();
    // The gloss lies lengthwise, so its top long side is 9 mm above its centre.
    let center = insert.placements[0].center_mm;
    assert!(notch.max.y <= center.y - 9.0 + ACCURACY_MM, "{notch:?}");
    assert!((notch.center().x - center.x).abs() < ACCURACY_MM);
}

#[test]
fn both_sides_can_get_a_notch() {
    let gloss = boxy("gloss", (18.0, 90.0));
    let both = FingerNotches {
        side: NotchSide::Both,
        ..FingerNotches::default()
    };
    let plain = generate(&design(vec![gloss.clone()], card(10.0, false)));
    let insert = generate(&design(vec![gloss.clone()], notched_card(both)));
    assert_eq!(notch_area((&gloss, &insert)).regions().len(), 2);
    let deeper = insert.width_mm - plain.width_mm;
    assert!(
        (deeper - 2.0 * FINGER_NOTCH_RADIUS_MM).abs() < ACCURACY_MM,
        "{deeper}"
    );
}

#[test]
fn notch_moves_along_the_side() {
    let gloss = boxy("gloss", (18.0, 90.0));
    let quarter = FingerNotches {
        along: 0.25,
        ..FingerNotches::default()
    };
    let insert = generate(&design(vec![gloss.clone()], notched_card(quarter)));
    let notch = notch_area((&gloss, &insert)).bounds().unwrap();
    // A quarter of the way along the 90 mm side, from its left end.
    let expected_x = insert.placements[0].center_mm.x - 45.0 + 0.25 * 90.0;
    assert!(
        (notch.center().x - expected_x).abs() < ACCURACY_MM,
        "{notch:?}"
    );
}

#[test]
fn notch_position_outside_the_side_is_refused() {
    let past_the_end = FingerNotches {
        along: 1.5,
        ..FingerNotches::default()
    };
    let design = design(vec![round("jar", 40.0)], notched_card(past_the_end));
    assert!(matches!(
        design.generate_insert(),
        Err(GenerateError::Invalid(DesignError::FractionOutOfRange { field, .. }))
            if field == "insert.finger_notches.along"
    ));
}

#[test]
fn rows_can_sit_anywhere_in_a_fixed_box() {
    // Card 156.5 × 116.5 mm; the jar's hole is 42 mm across; margin 10 mm.
    let cases = [
        (Align::TopLeft, [31.0, 31.0]),
        (Align::Top, [78.25, 31.0]),
        (Align::Center, [78.25, 58.25]),
        (Align::BottomRight, [125.5, 85.5]),
    ];
    for (align, expected) in cases {
        let mut design = design(
            vec![round("jar", 40.0)],
            CutoutCardSpec {
                align,
                ..card(10.0, false)
            },
        );
        design.outer_box = fixed_box(160.0, 120.0);
        let center = generate(&design).placements[0].center_mm;
        assert!(
            center.distance_to(Point::from(expected)) < ACCURACY_MM,
            "{align:?}: {center:?}"
        );
    }
}

#[test]
fn rows_line_up_the_way_the_block_does() {
    // Two 92 mm tube holes don't share a 140.5 mm row, so the rows are a tube, then a
    // tube and a jar. Aligned left, both rows start at the margin.
    let mut tube = boxy("tube", (90.0, 18.0));
    tube.quantity = 2;
    let mut design = design(
        vec![tube, round("jar", 40.0)],
        CutoutCardSpec {
            align: Align::Left,
            ..card(8.0, false)
        },
    );
    design.outer_box = fixed_box(160.0, 120.0);
    let insert = generate(&design);
    let lefts: Vec<f64> = holes(&insert)
        .iter()
        .map(|hole| hole.bounds().unwrap().min.x)
        .collect();
    let starts = lefts
        .iter()
        .filter(|&&left| (left - 8.0).abs() < ACCURACY_MM)
        .count();
    assert_eq!(starts, 2, "{lefts:?}");
}

#[test]
fn notch_turns_with_a_placed_product() {
    // The gloss outline stands upright (18 wide, 90 tall). Placed without a turn it stays
    // upright, so the notch for its "bottom" long side ends up on its right.
    let gloss = boxy("gloss", (18.0, 90.0));
    let mut spec = card(10.0, true);
    spec.placements = vec![placement("gloss", [60.0, 60.0])];
    let insert = generate(&design(vec![gloss.clone()], spec));

    let center = insert.placements[0].center_mm;
    let notch = notch_area((&gloss, &insert)).bounds().unwrap();
    assert!(notch.min.x >= center.x + 9.0 - ACCURACY_MM, "{notch:?}");
    assert!((notch.center().y - center.y).abs() < ACCURACY_MM);
}
