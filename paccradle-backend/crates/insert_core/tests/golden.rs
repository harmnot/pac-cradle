//! M1-A9: golden tests. Every insert style is drawn for 5 sample products, each in an
//! outer box, and compared with drawings a person has checked once (`tests/golden/`).
//! Any change to a generated piece, the layout, the fit or the warnings shows up here.
//!
//! - A changed drawing fails the test. The new one goes to the target folder, and the
//!   failure names it and the first line that differs.
//! - When a change is meant, look at the new drawings, then accept them all with
//!   `UPDATE_GOLDEN=1 cargo test -p insert_core --test golden`.
//! - Numbers compare within 0.002 mm, so last-digit rounding between machines passes
//!   (`common::golden`).
// The helpers below are test code too; clippy.toml only exempts `#[test]` functions.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use insert_core::{
    Align, Board, BoxSpec, BoxStyle, CradlePlatformSpec, CutoutCardSpec, Design, FingerNotches,
    InsertSpec, Material, NotchSide, PartitionGridSpec, Point, Polygon, Product,
};

/// One sample product, and how each insert style holds it.
struct Sample {
    name: &'static str,
    outline: Polygon,
    height_mm: f64,
    grid: GridSample,
    card: CardSample,
    cradle: CradleSample,
}

struct GridSample {
    board: Board,
    /// Rows, columns and how many cells hold a product.
    cells: (u32, u32, u32),
    divider_height_mm: f64,
    outer_frame: bool,
    finger_notches: bool,
    outer_box: (BoxStyle, Board),
}

struct CardSample {
    board: Board,
    quantity: u32,
    finger_notches: Option<FingerNotches>,
    outer_box: (BoxStyle, Board),
}

struct CradleSample {
    board: Board,
    quantity: u32,
    deck_height_mm: f64,
    outer_box: (BoxStyle, Board),
}

/// The five products: the shapes small brands send most (CLAUDE.md §2), from a square
/// bottle to a non-convex outline, across all five boards and both boxes.
fn samples() -> Vec<Sample> {
    use BoxStyle::{Mailer, Shipping};
    vec![
        Sample {
            name: "perfume",
            outline: common::rectangle(52.0, 38.0),
            height_mm: 110.0,
            grid: GridSample {
                board: Board::EFlute,
                cells: (2, 2, 4),
                divider_height_mm: 70.0,
                outer_frame: false,
                finger_notches: true,
                outer_box: (Shipping, Board::BFlute),
            },
            card: CardSample {
                board: Board::Chipboard,
                quantity: 2,
                finger_notches: Some(FingerNotches::default()),
                outer_box: (Mailer, Board::EFlute),
            },
            cradle: CradleSample {
                board: Board::Chipboard,
                quantity: 1,
                deck_height_mm: 40.0,
                outer_box: (Mailer, Board::EFlute),
            },
        },
        Sample {
            name: "candle",
            outline: common::circle(72.0),
            height_mm: 85.0,
            grid: GridSample {
                board: Board::BFlute,
                cells: (2, 3, 6),
                divider_height_mm: 60.0,
                outer_frame: true,
                finger_notches: true,
                outer_box: (Shipping, Board::CFlute),
            },
            card: CardSample {
                board: Board::EFlute,
                quantity: 2,
                finger_notches: None,
                outer_box: (Shipping, Board::BFlute),
            },
            cradle: CradleSample {
                board: Board::EFlute,
                quantity: 2,
                deck_height_mm: 35.0,
                outer_box: (Shipping, Board::BFlute),
            },
        },
        Sample {
            name: "lipstick",
            outline: common::circle(22.0),
            height_mm: 80.0,
            grid: GridSample {
                board: Board::Chipboard,
                cells: (3, 4, 12),
                divider_height_mm: 50.0,
                outer_frame: false,
                finger_notches: false,
                outer_box: (Mailer, Board::EFlute),
            },
            card: CardSample {
                board: Board::Paperboard,
                quantity: 6,
                finger_notches: Some(FingerNotches {
                    side: NotchSide::Both,
                    along: 0.5,
                }),
                outer_box: (Mailer, Board::EFlute),
            },
            cradle: CradleSample {
                board: Board::Paperboard,
                quantity: 3,
                deck_height_mm: 30.0,
                outer_box: (Mailer, Board::EFlute),
            },
        },
        Sample {
            name: "soap",
            outline: oval(90.0, 58.0),
            height_mm: 28.0,
            grid: GridSample {
                board: Board::EFlute,
                cells: (2, 2, 3),
                divider_height_mm: 20.0,
                outer_frame: true,
                finger_notches: false,
                outer_box: (Mailer, Board::EFlute),
            },
            card: CardSample {
                board: Board::Paperboard,
                quantity: 3,
                finger_notches: Some(FingerNotches {
                    side: NotchSide::Top,
                    along: 0.25,
                }),
                outer_box: (Shipping, Board::EFlute),
            },
            cradle: CradleSample {
                board: Board::Chipboard,
                quantity: 2,
                deck_height_mm: 15.0,
                outer_box: (Shipping, Board::EFlute),
            },
        },
        Sample {
            name: "pump_bottle",
            outline: pump_bottle(),
            height_mm: 150.0,
            grid: GridSample {
                board: Board::CFlute,
                cells: (2, 3, 6),
                divider_height_mm: 90.0,
                outer_frame: false,
                finger_notches: true,
                outer_box: (Shipping, Board::CFlute),
            },
            card: CardSample {
                board: Board::Chipboard,
                quantity: 1,
                finger_notches: Some(FingerNotches::default()),
                outer_box: (Mailer, Board::BFlute),
            },
            cradle: CradleSample {
                board: Board::BFlute,
                quantity: 1,
                deck_height_mm: 50.0,
                outer_box: (Mailer, Board::BFlute),
            },
        },
    ]
}

/// An oval (ellipse) with 64 points, inside its bounding box from the origin: a soap bar.
fn oval(length_mm: f64, width_mm: f64) -> Polygon {
    let (a, b) = (length_mm / 2.0, width_mm / 2.0);
    let points = (0..64)
        .map(|step| {
            let angle = f64::from(step) * std::f64::consts::TAU / 64.0;
            Point::new(a + a * angle.cos(), b + b * angle.sin())
        })
        .collect();
    Polygon(points)
}

/// A round pump bottle seen from above: a Ø50 mm body with its nozzle sticking out 22 mm
/// to the right, 8 mm wide. Not convex, like many vision outlines.
fn pump_bottle() -> Polygon {
    let (radius, nozzle_half, nozzle_end) = (25.0_f64, 4.0_f64, 72.0);
    // Where the nozzle's sides meet the body, as an angle from +x.
    let joint = (nozzle_half / radius).asin();
    let body_steps = 60;
    let sweep = std::f64::consts::TAU - 2.0 * joint;
    let mut points: Vec<Point> = (0..=body_steps)
        .map(|step| {
            let angle = joint + sweep * f64::from(step) / f64::from(body_steps);
            Point::new(radius + radius * angle.cos(), radius + radius * angle.sin())
        })
        .collect();
    points.push(Point::new(nozzle_end, radius - nozzle_half));
    points.push(Point::new(nozzle_end, radius + nozzle_half));
    Polygon(points)
}

fn product((sample, quantity): (&Sample, u32)) -> Product {
    Product {
        id: sample.name.to_owned(),
        contour_mm: sample.outline.clone(),
        height_mm: sample.height_mm,
        quantity,
    }
}

fn outer_box((style, board): (BoxStyle, Board)) -> Option<BoxSpec> {
    Some(BoxSpec {
        style,
        material: Material::starting_values(board),
        inside_mm: None,
    })
}

fn grid_design(sample: &Sample) -> Design {
    let grid = &sample.grid;
    let (rows, columns, quantity) = grid.cells;
    Design {
        material: Material::starting_values(grid.board),
        products: vec![product((sample, quantity))],
        insert: InsertSpec::PartitionGrid(PartitionGridSpec {
            rows,
            columns,
            divider_height_mm: grid.divider_height_mm,
            outer_frame: grid.outer_frame,
            finger_notches: grid.finger_notches,
        }),
        outer_box: outer_box(grid.outer_box),
    }
}

fn card_design(sample: &Sample) -> Design {
    let card = &sample.card;
    Design {
        material: Material::starting_values(card.board),
        products: vec![product((sample, card.quantity))],
        insert: InsertSpec::CutoutCard(CutoutCardSpec {
            margin_mm: 10.0,
            finger_notches: card.finger_notches,
            align: Align::Center,
            placements: Vec::new(),
        }),
        outer_box: outer_box(card.outer_box),
    }
}

fn cradle_design(sample: &Sample) -> Design {
    let cradle = &sample.cradle;
    Design {
        material: Material::starting_values(cradle.board),
        products: vec![product((sample, cradle.quantity))],
        insert: InsertSpec::CradlePlatform(CradlePlatformSpec {
            deck_height_mm: cradle.deck_height_mm,
            margin_mm: 12.0,
            align: Align::Center,
            placements: Vec::new(),
        }),
        outer_box: outer_box(cradle.outer_box),
    }
}

/// Draws each sample with `make` and checks it against its golden drawing,
/// `tests/golden/<style>_<product>.svg`. Reports every sample that differs at once.
fn check_style((style, make): (&str, fn(&Sample) -> Design)) {
    let failures: Vec<String> = samples()
        .iter()
        .filter_map(|sample| {
            let name = format!("{style}_{}", sample.name);
            let title = format!("{name}: {} × {}", sample.name, style.replace('_', " "));
            let drawing = common::svg::render((&make(sample), &title));
            common::golden::check((&format!("{name}.svg"), &drawing)).err()
        })
        .collect();
    assert!(failures.is_empty(), "\n{}", failures.join("\n\n"));
}

#[test]
fn partition_grids_match_their_golden_drawings() {
    check_style(("partition_grid", grid_design));
}

#[test]
fn cutout_cards_match_their_golden_drawings() {
    check_style(("cutout_card", card_design));
}

#[test]
fn cradle_platforms_match_their_golden_drawings() {
    check_style(("cradle_platform", cradle_design));
}

#[test]
fn number_rounding_passes_and_real_changes_fail() {
    use common::golden::{first_difference, same_line};
    assert!(same_line(("M10.001 -3 L4.5 0", "M10 -3.002 L4.501 0")));
    assert!(!same_line(("M10.001 -3 L4.5 0", "M10.01 -3 L4.5 0")));
    assert!(!same_line(("M10 L4", "M10 L4 L5")));
    assert!(!same_line(("Green", "Yellow")));
    assert_eq!(first_difference(("a\nb 1\nc", "a\nb 1.001\nc")), None);
    assert_eq!(first_difference(("a\nb\nc", "a\nb")), Some(3));
    assert_eq!(common::svg::num(-0.0004), "0");
    assert_eq!(common::svg::num(12.5), "12.5");
}
