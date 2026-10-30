//! Design specs for the tests (M1-D6): one of each insert style.

use serde_json::{Value, json};

/// The documented example: a cradle for two 52 × 38 mm bottles in a mailer box.
pub fn cradle_spec() -> Value {
    serde_json::from_str(include_str!(
        "../../crates/insert_core/tests/fixtures/cradle_mailer.json"
    ))
    .unwrap()
}

/// Four 30 × 20 mm lipsticks in a card, no box.
pub fn card_spec() -> Value {
    json!({
        "material": { "board": "chipboard", "thickness_mm": 1.5, "clearance_mm": 1.0, "slot_tolerance_mm": 0.3 },
        "products": [{
            "id": "p1",
            "contour_mm": [[0, 0], [30, 0], [30, 20], [0, 20]],
            "height_mm": 15,
            "quantity": 4
        }],
        "insert": { "style": "cutout_card", "margin_mm": 12 },
        "outer_box": null
    })
}

/// Six 40 mm jars in a 2 × 3 grid, no box.
pub fn grid_spec() -> Value {
    json!({
        "material": { "board": "e_flute", "thickness_mm": 1.6, "clearance_mm": 1.5, "slot_tolerance_mm": 0.5 },
        "products": [{
            "id": "jar",
            "contour_mm": [[0, 0], [40, 0], [40, 40], [0, 40]],
            "height_mm": 100,
            "quantity": 6
        }],
        "insert": {
            "style": "partition_grid", "rows": 2, "columns": 3, "divider_height_mm": 80,
            "outer_frame": false, "finger_notches": false
        },
        "outer_box": null
    })
}
