//! `insert_core` for the browser preview (M1-A10, CLAUDE.md §6.1 and §7.4).
//!
//! JSON in, JSON out: the editor sends a `Design` as JSON and gets JSON back, in the same
//! shapes the engine's types serialize to. Drawn lines come back as plain straight
//! segments, within 0.05 mm of the true lines with points to 0.01 mm
//! ([`Packaging::rounded_for_preview`]); production files are made only on the server,
//! and this crate never gets the export writers.
//!
//! - [`generate`]: the packaging to draw and fold, on every slider frame.
//! - [`fits`]: each product copy's fit colour, cheap enough for every frame too.
//! - [`warnings`]: the manufacturability warnings; run it once an edit settles.
//!
//! Each call checks and generates the design again; nothing is kept between calls. A
//! design that can't be generated throws a JS `Error` whose message says what to fix.
//!
//! The `*_json` functions do the work and run natively too, for the tests; the exported
//! functions only turn their errors into JS errors.

use insert_core::{CheckOptions, Design, GenerateError, GeometryError, Packaging};
use serde::Serialize;
use thiserror::Error;
use wasm_bindgen::prelude::*;

/// Why a call failed. Its message becomes the JS error's.
#[derive(Debug, Error)]
pub enum PreviewError {
    /// The input isn't JSON for what the call takes.
    #[error("not a valid {what}: {source}")]
    Input {
        what: &'static str,
        source: serde_json::Error,
    },
    #[error(transparent)]
    Generate(#[from] GenerateError),
    #[error(transparent)]
    Geometry(#[from] GeometryError),
    /// The result couldn't be written as JSON.
    #[error("couldn't write the result: {0}")]
    Output(serde_json::Error),
}

/// The packaging for `design_json` (a `Design`), as JSON, rounded for the preview.
#[wasm_bindgen]
pub fn generate(design_json: &str) -> Result<String, JsError> {
    to_js(generate_json(design_json))
}

/// Every product copy's fit (a `ProductFit` list), as JSON, in placement order.
#[wasm_bindgen]
pub fn fits(design_json: &str) -> Result<String, JsError> {
    to_js(fits_json(design_json))
}

/// The manufacturability warnings (a `Warning` list), as JSON. `options_json` is a
/// `CheckOptions`; `{}` checks without a sheet size.
#[wasm_bindgen]
pub fn warnings(design_json: &str, options_json: &str) -> Result<String, JsError> {
    to_js(warnings_json((design_json, options_json)))
}

/// [`generate`]'s work.
pub fn generate_json(design_json: &str) -> Result<String, PreviewError> {
    let (_, packaging) = generated(design_json)?;
    to_json(&packaging.rounded_for_preview())
}

/// [`fits`]' work.
pub fn fits_json(design_json: &str) -> Result<String, PreviewError> {
    let (design, packaging) = generated(design_json)?;
    to_json(&packaging.fits(&design)?)
}

/// [`warnings`]' work.
pub fn warnings_json((design_json, options_json): (&str, &str)) -> Result<String, PreviewError> {
    let options: CheckOptions = parse((options_json, "check options"))?;
    let (design, packaging) = generated(design_json)?;
    to_json(&packaging.warnings((&design, &options))?)
}

/// The design read from JSON, and what it generates (exact, not rounded).
fn generated(design_json: &str) -> Result<(Design, Packaging), PreviewError> {
    let design: Design = parse((design_json, "design"))?;
    let packaging = design.generate()?;
    Ok((design, packaging))
}

fn parse<T: serde::de::DeserializeOwned>(
    (json, what): (&str, &'static str),
) -> Result<T, PreviewError> {
    serde_json::from_str(json).map_err(|source| PreviewError::Input { what, source })
}

fn to_json(value: &impl Serialize) -> Result<String, PreviewError> {
    serde_json::to_string(value).map_err(PreviewError::Output)
}

fn to_js(result: Result<String, PreviewError>) -> Result<String, JsError> {
    result.map_err(|error| JsError::new(&error.to_string()))
}
