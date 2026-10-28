//! M1-A1 "done when": designs and pieces survive a JSON round trip unchanged, and the
//! JSON keeps the documented shape (`tests/fixtures/cradle_mailer.json`).

mod common;

use insert_core::{
    Board, Design, FoldDirection, Hinge, Line, Material, Panel, Path, Piece, Point, Segment,
};
use serde_json::{Value, json};

const CRADLE_MAILER_JSON: &str = include_str!("fixtures/cradle_mailer.json");

/// Writes `value` as JSON text and reads it back, as the API does with `designs.spec`.
fn round_trip<T>(value: &T) -> Result<T, serde_json::Error>
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    let text = serde_json::to_string(value)?;
    serde_json::from_str(&text)
}

#[test]
fn every_insert_style_round_trips() {
    for design in [
        common::grid_design(),
        common::card_design(),
        common::cradle_design(),
    ] {
        assert_eq!(round_trip(&design).unwrap(), design);
    }
}

#[test]
fn fixture_parses_validates_and_serializes_back_unchanged() {
    let original: Value = serde_json::from_str(CRADLE_MAILER_JSON).unwrap();
    let design: Design = serde_json::from_value(original.clone()).unwrap();

    assert_eq!(design.validate(), Ok(()));
    assert_eq!(serde_json::to_value(&design).unwrap(), original);
}

#[test]
fn json_uses_the_documented_names() {
    let value = serde_json::to_value(common::grid_design()).unwrap();

    assert_eq!(value["insert"]["style"], "partition_grid");
    assert_eq!(value["insert"]["rows"], 2);
    assert_eq!(value["material"]["board"], "e_flute");
    assert_eq!(value["outer_box"], Value::Null);
    let first_point = &value["products"][0]["contour_mm"][0];
    assert!(
        first_point.is_array(),
        "points are [x, y] arrays like the vision output"
    );
    assert_eq!(first_point.as_array().map(Vec::len), Some(2));
}

#[test]
fn optional_fields_can_be_left_out() {
    let design: Design = serde_json::from_value(json!({
        "material": { "board": "paperboard", "thickness_mm": 0.5, "clearance_mm": 1.0, "slot_tolerance_mm": 0.3 },
        "products": [{ "id": "p1", "contour_mm": [[0.0, 0.0], [20.0, 0.0], [20.0, 20.0]], "height_mm": 10.0, "quantity": 1 }],
        "insert": { "style": "cutout_card", "margin_mm": 8.0, "finger_notches": false }
    }))
    .unwrap();

    assert_eq!(design.outer_box, None);
    let insert_core::InsertSpec::CutoutCard(card) = &design.insert else {
        panic!("expected a cutout card, got {:?}", design.insert);
    };
    assert!(card.placements.is_empty());
}

#[test]
fn styles_outside_the_mvp_are_rejected() {
    let result =
        serde_json::from_value::<insert_core::InsertSpec>(json!({ "style": "pocket_tray" }));
    assert!(result.is_err());
}

#[test]
fn piece_with_arcs_panels_and_hinges_round_trips() {
    let deck = Path {
        start: Point::new(0.0, 0.0),
        segments: vec![
            Segment::Line {
                to: Point::new(100.0, 0.0),
            },
            Segment::Line {
                to: Point::new(100.0, 60.0),
            },
            Segment::Line {
                to: Point::new(0.0, 60.0),
            },
        ],
        closed: true,
    };
    let finger_hole = Path {
        start: Point::new(60.0, 30.0),
        segments: vec![Segment::Arc {
            center: Point::new(50.0, 30.0),
            sweep_deg: 360.0,
        }],
        closed: true,
    };
    let crease = Path {
        start: Point::new(0.0, 20.0),
        segments: vec![Segment::Line {
            to: Point::new(100.0, 20.0),
        }],
        closed: false,
    };
    let piece = Piece {
        name: "deck".to_owned(),
        quantity: 1,
        outline: deck,
        holes: vec![finger_hole],
        creases: vec![crease],
        perforations: Vec::new(),
        panels: vec![
            Panel {
                outline: common::rectangle(100.0, 20.0),
                holes: Vec::new(),
            },
            Panel {
                outline: common::rectangle(100.0, 40.0),
                holes: vec![common::circle(20.0)],
            },
        ],
        hinges: vec![Hinge {
            parent: 0,
            child: 1,
            axis: Line {
                from: Point::new(0.0, 20.0),
                to: Point::new(100.0, 20.0),
            },
            fold_deg: 90.0,
            direction: FoldDirection::Valley,
            stage: 0,
        }],
    };

    assert_eq!(round_trip(&piece).unwrap(), piece);
    let value = serde_json::to_value(&piece).unwrap();
    assert_eq!(value["holes"][0]["segments"][0]["kind"], "arc");
    assert_eq!(value["hinges"][0]["direction"], "valley");
}

#[test]
fn starting_values_follow_the_material_table() {
    for board in Board::ALL {
        let material = Material::starting_values(board);
        assert!(
            board.typical_thickness().contains(material.thickness_mm),
            "{board:?} starts outside its typical thickness"
        );
        let expected_tolerance = if board.is_corrugated() { 0.5 } else { 0.3 };
        assert_eq!(material.slot_tolerance_mm, expected_tolerance, "{board:?}");
        assert!(material.clearance_mm >= 1.0, "{board:?}");
    }
}
