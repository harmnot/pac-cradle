//! M1-A10: the browser calls, run natively. JSON in, JSON out: the packaging comes back
//! rounded for the preview, fits and warnings as the engine gives them, and a design
//! that can't be generated gives a message saying what to fix.
// The helpers below are test code too; clippy.toml only exempts `#[test]` functions.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use insert_core::{Design, PREVIEW_GRID_MM, Packaging, ProductFit, Segment, Warning};
use insert_core_wasm::{fits_json, generate_json, warnings_json};

/// The engine's own sample: two bottles on a chipboard cradle in an E-flute mailer.
const DESIGN: &str = include_str!("../../insert_core/tests/fixtures/cradle_mailer.json");

#[test]
fn generate_returns_the_packaging_rounded_for_the_preview() {
    let design: Design = serde_json::from_str(DESIGN).unwrap();
    let exact = design.generate().unwrap();

    let preview: Packaging = serde_json::from_str(&generate_json(DESIGN).unwrap()).unwrap();
    assert_eq!(preview, exact.rounded_for_preview());

    let deck = &preview.insert.pieces[0];
    for segment in &deck.outline.segments {
        let Segment::Line { to } = segment else {
            panic!("an arc in the preview");
        };
        let steps = to.x / PREVIEW_GRID_MM;
        assert!(
            (steps - steps.round()).abs() < 1e-6,
            "{to:?} is off the grid"
        );
    }
}

#[test]
fn fits_and_warnings_come_back_as_the_engine_gives_them() {
    let design: Design = serde_json::from_str(DESIGN).unwrap();
    let packaging = design.generate().unwrap();

    let fits: Vec<ProductFit> = serde_json::from_str(&fits_json(DESIGN).unwrap()).unwrap();
    assert_eq!(fits, packaging.fits(&design).unwrap());
    assert_eq!(fits.len(), 2);

    let options = r#"{ "sheet": { "length_mm": 100.0, "width_mm": 100.0 } }"#;
    let warnings: Vec<Warning> =
        serde_json::from_str(&warnings_json((DESIGN, options)).unwrap()).unwrap();
    let expected = packaging
        .warnings((&design, &serde_json::from_str(options).unwrap()))
        .unwrap();
    assert_eq!(warnings, expected);
    // The cradle and the mailer are both bigger than a 100 mm sheet.
    assert!(!warnings.is_empty());

    // No sheet: options can leave it out.
    assert!(warnings_json((DESIGN, "{}")).is_ok());
}

#[test]
fn bad_input_says_what_is_wrong() {
    let not_json = generate_json("{").unwrap_err().to_string();
    assert!(not_json.starts_with("not a valid design:"), "{not_json}");

    let options = warnings_json((DESIGN, "[]")).unwrap_err().to_string();
    assert!(
        options.starts_with("not a valid check options:"),
        "{options}"
    );

    // Valid JSON, but a design the engine refuses: its own message comes through.
    let mut design: serde_json::Value = serde_json::from_str(DESIGN).unwrap();
    design["products"] = serde_json::json!([]);
    let refused = fits_json(&design.to_string()).unwrap_err().to_string();
    assert_eq!(refused, "a design needs at least one product");
}
