//! M1-A4 "done when": the crossing slots line up. Checked from the generated pieces and
//! where they stand, not from the generator's own numbers. Also: strip counts and sizes,
//! the frame, cells that hold the product, boxes of fixed size, and finger notches.
// The helpers below are test code too; clippy.toml only exempts `#[test]` functions.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::measure::{is_inside, rectangle_at};
use insert_core::{
    Area, Board, BoxSize, BoxSpec, BoxStyle, Design, GenerateError, Insert, InsertSpec, Material,
    Mount, PartitionGridSpec, Piece, Point, Product, Stance, Transform,
};

/// E-flute starting values (CLAUDE.md §6.4).
const THICKNESS_MM: f64 = 1.6;
const CLEARANCE_MM: f64 = 1.5;
const SLOT_WIDTH_MM: f64 = THICKNESS_MM + 0.5;
/// E-flute's narrowest web: 3× 1.6 mm.
const WEB_MM: f64 = 4.8;
const HEIGHT_MM: f64 = 60.0;
const SLOT_DEPTH_MM: f64 = HEIGHT_MM / 2.0 + 0.5;
/// Height steps sampled at each crossing.
const SAMPLES: u32 = 240;
/// Slack for sizes read back from cut outlines: the polygon library snaps to 0.1 µm.
const GRID_SLACK_MM: f64 = 1e-3;

/// Square tins 72 mm a side: they lie straight as drawn, so cell sizes come out exact.
fn tins(quantity: u32) -> Product {
    let mut tin = common::product("tin", rectangle_at(Point::ORIGIN, (72.0, 72.0)));
    tin.quantity = quantity;
    tin
}

/// Round jars 72 mm across (a 64-sided outline, like a vision contour).
fn jars(quantity: u32) -> Product {
    let mut jar = common::product("jar", common::circle(72.0));
    jar.quantity = quantity;
    jar
}

fn grid((rows, columns): (u32, u32), (outer_frame, finger_notches): (bool, bool)) -> InsertSpec {
    InsertSpec::PartitionGrid(PartitionGridSpec {
        rows,
        columns,
        divider_height_mm: HEIGHT_MM,
        outer_frame,
        finger_notches,
    })
}

fn design(product: Product, insert: InsertSpec) -> Design {
    Design {
        material: Material::starting_values(Board::EFlute),
        products: vec![product],
        insert,
        outer_box: None,
    }
}

/// A box with a fixed inside size, of B-flute (1.75 mm clearance).
fn fixed_box((length_mm, width_mm): (f64, f64)) -> Option<BoxSpec> {
    Some(BoxSpec {
        style: BoxStyle::Shipping,
        material: Material::starting_values(Board::BFlute),
        inside_mm: Some(BoxSize {
            length_mm,
            width_mm,
            height_mm: 80.0,
        }),
    })
}

fn generate(design: &Design) -> Insert {
    match design.generate_insert() {
        Ok(insert) => insert,
        Err(error) => panic!("generate: {error}"),
    }
}

/// True when the flat piece has board at `point` (u along it, v down from its top edge).
fn solid(piece: &Piece, point: Point) -> bool {
    let panel = &piece.panels[0];
    is_inside(point, &panel.outline) && !panel.holes.iter().any(|hole| is_inside(point, hole))
}

fn mounts_of(insert: &Insert, stance: Stance) -> Vec<Mount> {
    insert
        .mounts
        .iter()
        .copied()
        .filter(|mount| mount.stance == stance)
        .collect()
}

/// Checks one crossing of a long strip (along x) and a short strip (along y) in the
/// assembled grid. Returns how many height samples had board from each strip.
fn check_crossing((insert, long, short): (&Insert, Mount, Mount)) -> (u32, u32) {
    let long_piece = &insert.pieces[long.piece];
    let short_piece = &insert.pieces[short.piece];
    // Where the strips meet, measured along each one.
    let along_long = short.at_mm[0] - long.at_mm[0];
    let along_short = long.at_mm[1] - short.at_mm[1];
    let (mut long_board, mut short_board) = (0, 0);
    for step in 0..SAMPLES {
        let z = HEIGHT_MM * (f64::from(step) + 0.5) / f64::from(SAMPLES);
        let long_v = long.at_mm[2] - z;
        let short_v = short.at_mm[2] - z;
        // Each strip's board at the crossing, across the other strip's full thickness.
        let half = THICKNESS_MM / 2.0 - 1e-6;
        let long_has = [-half, 0.0, half]
            .iter()
            .any(|&d| solid(long_piece, Point::new(along_long + d, long_v)));
        let short_has = [-half, 0.0, half]
            .iter()
            .any(|&d| solid(short_piece, Point::new(along_short + d, short_v)));
        assert!(
            !(long_has && short_has),
            "strips collide at x {}, y {}, height {z:.2}",
            short.at_mm[0],
            long.at_mm[1]
        );
        long_board += u32::from(long_has);
        short_board += u32::from(short_has);
    }
    (long_board, short_board)
}

#[test]
fn crossing_slots_line_up() {
    for (cells, options) in [
        ((2, 3), (false, false)),
        ((2, 3), (false, true)),
        ((3, 4), (true, false)),
        ((3, 4), (true, true)),
    ] {
        let insert = generate(&design(tins(6), grid(cells, options)));
        let longs = mounts_of(&insert, Stance::UprightAlongX);
        let shorts = mounts_of(&insert, Stance::UprightAlongY);
        let extra = if options.0 { 1 } else { -1 };
        assert_eq!(longs.len() as i64, i64::from(cells.0) + extra);
        assert_eq!(shorts.len() as i64, i64::from(cells.1) + extra);
        for &long in &longs {
            for &short in &shorts {
                let (long_board, short_board) = check_crossing((&insert, long, short));
                // Both strips hold at every crossing: each keeps board over most of a half.
                let half = SAMPLES / 2;
                assert!(long_board > half - 4 && long_board < half, "{long_board}");
                assert!(
                    short_board > half - 4 && short_board < half,
                    "{short_board}"
                );
            }
        }
    }
}

#[test]
fn slots_are_one_board_plus_tolerance_wide_and_just_over_half_deep() {
    let insert = generate(&design(tins(6), grid((2, 3), (false, false))));
    let long = &insert.pieces[0];
    assert_eq!(long.name, "long divider");
    // The long divider's first slot, cut up from the bottom at the first crossing.
    let at = 75.0 + THICKNESS_MM / 2.0;
    let in_slot = HEIGHT_MM - SLOT_DEPTH_MM / 2.0;
    assert!(!solid(long, Point::new(at, in_slot)));
    assert!(!solid(
        long,
        Point::new(at - SLOT_WIDTH_MM / 2.0 + 0.01, in_slot)
    ));
    assert!(solid(
        long,
        Point::new(at - SLOT_WIDTH_MM / 2.0 - 0.01, in_slot)
    ));
    assert!(!solid(
        long,
        Point::new(at, HEIGHT_MM - SLOT_DEPTH_MM + 0.01)
    ));
    assert!(solid(
        long,
        Point::new(at, HEIGHT_MM - SLOT_DEPTH_MM - 0.01)
    ));
}

#[test]
fn strips_and_cells_are_sized_from_the_product() {
    let insert = generate(&design(tins(6), grid((2, 3), (false, false))));
    // Each cell is the 72 mm tin plus 1.5 mm clearance a side; one board between cells.
    let cell = 72.0 + 2.0 * CLEARANCE_MM;
    let length = 3.0 * cell + 2.0 * THICKNESS_MM;
    let width = 2.0 * cell + THICKNESS_MM;
    assert!((insert.length_mm - length).abs() < 1e-6);
    assert!((insert.width_mm - width).abs() < 1e-6);
    assert_eq!(insert.height_mm, HEIGHT_MM);

    let names: Vec<(&str, u32)> = insert
        .pieces
        .iter()
        .map(|piece| (piece.name.as_str(), piece.quantity))
        .collect();
    assert_eq!(names, vec![("long divider", 1), ("short divider", 2)]);
    assert_eq!(insert.mounts.len(), 3);
    let long_bounds = insert.pieces[0].panels[0].outline.bounds().unwrap();
    assert!((long_bounds.length_mm() - length).abs() < GRID_SLACK_MM);
    let short_bounds = insert.pieces[1].panels[0].outline.bounds().unwrap();
    assert!((short_bounds.length_mm() - width).abs() < GRID_SLACK_MM);
}

#[test]
fn every_product_sits_in_its_own_cell_with_clearance() {
    let product = jars(5);
    let insert = generate(&design(product.clone(), grid((2, 3), (false, false))));
    assert_eq!(
        insert.placements.len(),
        5,
        "one per jar; one cell stays empty"
    );

    let pivot = product.contour_mm.bounds().unwrap().center();
    for placement in &insert.placements {
        let place = Transform::shift(Point::ORIGIN - pivot).then(Transform {
            turn_deg: placement.rotation_deg,
            shift: placement.center_mm,
        });
        let outline = product.contour_mm.transformed(place);
        for mount in &insert.mounts {
            let (across, wall) = match mount.stance {
                Stance::UprightAlongX => (1, mount.at_mm[1]),
                _ => (0, mount.at_mm[0]),
            };
            for point in outline.points() {
                let coordinate = [point.x, point.y][across];
                let gap = (coordinate - wall).abs() - THICKNESS_MM / 2.0;
                assert!(gap >= CLEARANCE_MM - 1e-6, "{gap} mm from a strip");
            }
        }
        let bounds = outline.bounds().unwrap();
        assert!(bounds.min.x >= CLEARANCE_MM - 1e-6 && bounds.min.y >= CLEARANCE_MM - 1e-6);
        assert!(bounds.max.x <= insert.length_mm - CLEARANCE_MM + 1e-6);
        assert!(bounds.max.y <= insert.width_mm - CLEARANCE_MM + 1e-6);
    }
    // Row by row from the top-left: the first jar sits in the first cell, the fourth
    // starts the second row.
    let (first, fourth) = (
        insert.placements[0].center_mm,
        insert.placements[3].center_mm,
    );
    assert!((first.x - fourth.x).abs() < 1e-6 && fourth.y > first.y);
}

#[test]
fn first_tin_sits_in_the_first_cell() {
    let insert = generate(&design(tins(5), grid((2, 3), (false, false))));
    let first = insert.placements[0].center_mm;
    assert!(first.distance_to(Point::new(37.5, 37.5)) < 1e-6);
}

#[test]
fn frame_runs_round_the_outside_with_board_past_the_corners() {
    let plain = generate(&design(tins(6), grid((2, 3), (false, false))));
    let framed = generate(&design(tins(6), grid((2, 3), (true, false))));

    let names: Vec<(&str, u32)> = framed
        .pieces
        .iter()
        .map(|piece| (piece.name.as_str(), piece.quantity))
        .collect();
    assert_eq!(names, vec![("long divider", 3), ("short divider", 4)]);
    // A board each side, plus the end tab past each corner crossing.
    let end_tab = SLOT_WIDTH_MM / 2.0 + WEB_MM;
    let extra = THICKNESS_MM + 2.0 * end_tab;
    assert!((framed.length_mm - (plain.length_mm + extra)).abs() < 1e-6);
    assert!((framed.width_mm - (plain.width_mm + extra)).abs() < 1e-6);

    // The corner slot keeps a full web of board between it and the strip's end.
    let long = &framed.pieces[0];
    let in_slot = HEIGHT_MM - 1.0;
    assert!(solid(long, Point::new(WEB_MM - 0.01, in_slot)));
    assert!(!solid(long, Point::new(WEB_MM + 0.01, in_slot)));
}

#[test]
fn fixed_box_stretches_the_cells_to_fill_it() {
    let mut design = design(tins(6), grid((2, 3), (false, false)));
    design.outer_box = fixed_box((250.0, 170.0));
    let insert = generate(&design);
    // Inside size minus the box's 1.75 mm clearance on each side.
    assert!((insert.length_mm - 246.5).abs() < 1e-6);
    assert!((insert.width_mm - 166.5).abs() < 1e-6);
    let cell = (246.5 - 2.0 * THICKNESS_MM) / 3.0;
    let first = insert.placements[0].center_mm;
    assert!((first.x - cell / 2.0).abs() < 1e-6);
}

#[test]
fn box_too_small_for_the_cells_is_reported() {
    let mut design = design(tins(6), grid((2, 3), (false, false)));
    design.outer_box = fixed_box((200.0, 170.0));
    let Err(GenerateError::DoesNotFit {
        needed_length_mm,
        room_length_mm,
        ..
    }) = design.generate_insert()
    else {
        panic!("expected DoesNotFit");
    };
    assert!((needed_length_mm - (3.0 * 75.0 + 2.0 * THICKNESS_MM)).abs() < 1e-6);
    assert_eq!(room_length_mm, 196.5);
}

#[test]
fn long_products_turn_crosswise_when_only_that_fits() {
    // Two 80 × 30 tubes side by side along x need 166 mm lying lengthwise, but stood
    // crosswise they need 2 × 33 + 1.6 = 67.6 mm.
    let mut tube = common::product("tube", rectangle_at(Point::ORIGIN, (80.0, 30.0)));
    tube.quantity = 2;
    let mut design = design(tube.clone(), grid((1, 2), (false, false)));
    design.outer_box = fixed_box((75.0, 120.0));
    let insert = generate(&design);
    let pivot = tube.contour_mm.bounds().unwrap().center();
    let placement = &insert.placements[0];
    let place = Transform::shift(Point::ORIGIN - pivot).then(Transform {
        turn_deg: placement.rotation_deg,
        shift: placement.center_mm,
    });
    let bounds = tube.contour_mm.transformed(place).bounds().unwrap();
    assert!(bounds.width_mm() > bounds.length_mm(), "{bounds:?}");
}

#[test]
fn finger_notches_dip_into_every_cell_wall() {
    let plain = generate(&design(tins(6), grid((2, 3), (false, false))));
    let notched = generate(&design(tins(6), grid((2, 3), (false, true))));
    let area = |piece: &Piece| Area::from_polygon(&piece.panels[0].outline).unwrap();
    // The long divider runs past 3 cells, the short dividers past 2.
    for (index, cells) in [(0, 3), (1, 2)] {
        let removed = area(&plain.pieces[index])
            .difference(&area(&notched.pieces[index]))
            .unwrap();
        assert_eq!(removed.regions().len(), cells, "piece {index}");
        // Each notch is centred on its cell wall, in the strip's top edge.
        for region in removed.regions() {
            let bounds = region.outer.bounds().unwrap();
            assert!(
                bounds.min.y.abs() < GRID_SLACK_MM,
                "notches open from the top edge"
            );
        }
    }
}

#[test]
fn a_single_row_has_only_cross_dividers() {
    let insert = generate(&design(tins(3), grid((1, 3), (false, false))));
    let names: Vec<(&str, u32)> = insert
        .pieces
        .iter()
        .map(|piece| (piece.name.as_str(), piece.quantity))
        .collect();
    assert_eq!(names, vec![("short divider", 2)]);
    assert!(
        insert.mounts.iter().all(|mount| mount.piece == 0),
        "mounts point at the remaining piece"
    );
}

#[test]
fn one_cell_needs_a_frame() {
    assert_eq!(
        design(tins(1), grid((1, 1), (false, false))).generate_insert(),
        Err(GenerateError::EmptyGrid)
    );
    let framed = generate(&design(tins(1), grid((1, 1), (true, false))));
    assert_eq!(framed.pieces.len(), 2);
    assert_eq!(framed.mounts.len(), 4);
}

#[test]
fn grid_insert_survives_a_json_round_trip() {
    let insert = generate(&design(tins(6), grid((2, 3), (true, true))));
    let json = serde_json::to_string(&insert).unwrap();
    let back: Insert = serde_json::from_str(&json).unwrap();
    assert_eq!(back, insert);
}
