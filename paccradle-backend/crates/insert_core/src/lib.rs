//! Paccradle geometry core (CLAUDE.md §6).
//!
//! Generates insert pieces (partition grid, cutout card, cradle platform),
//! outer boxes, hinges for the 3D fold, fit checks and manufacturability
//! warnings. All units are millimetres.
//!
//! The same crate is compiled natively (API exports) and to WASM (browser
//! preview); export writers stay behind a server-only feature flag.
//!
//! Data model (M1-A1): a [`Design`] is the input a user edits. Check it with
//! [`Design::validate`]; generators (M1-A3 to A6) turn it into [`Piece`]s.
//!
//! Geometry helpers (M1-A2) the generators build on: [`Path::flatten`],
//! [`Polygon::simplify`], and [`Area`] for offsets and booleans.
//!
//! Generators (M1-A3 onwards): [`Design::generate`] returns the [`Packaging`]: the
//! [`Insert`] and, if the design has one, the [`OuterBox`] round it (M1-A6).
//! [`Design::generate_insert`] makes the insert alone.
//!
//! Manufacturability checks (M1-A7): [`Packaging::warnings`] lists what a printer or die
//! maker would flag, each [`Warning`] pointing at the piece and the spot.
//!
//! Fit check (M1-A8): [`Packaging::fits`] colours every product copy green, yellow or red
//! by the room round it, with any [`Clash`] named.
//!
//! Browser preview (M1-A10): [`Packaging::rounded_for_preview`] is the copy the editor
//! gets: straight segments within [`PREVIEW_TOLERANCE_MM`], on the [`PREVIEW_GRID_MM`]
//! grid (CLAUDE.md §7.4). The `insert_core_wasm` crate
//! wraps this crate for the browser.
//!
//! Production files (M1-B), behind the `export` feature: [`export::Sheet`] lays out every
//! piece once, box pieces mirrored print side up, and writes it as SVG
//! ([`Packaging::to_svg`]). The free print-at-home test PDF (M1-B4,
//! `Packaging::to_test_print`) tiles every copy over A4 or Letter pages from the preview's
//! geometry. The browser build never turns the feature on.

pub mod check;
pub mod design;
pub mod error;
#[cfg(feature = "export")]
pub mod export;
pub mod fit;
mod folding;
pub mod geometry;
pub mod insert;
pub mod limits;
pub mod material;
pub mod outer_box;
pub mod packaging;
pub mod piece;
pub mod preview;
mod validate;

pub use check::{CheckOptions, Part, Rule, SheetSize, SmallFeature, Spot, Warning};
pub use design::{
    Align, BoxSize, BoxSpec, BoxStyle, CradlePlatformSpec, CutoutCardSpec, Design, FingerNotches,
    InsertSpec, NotchSide, PartitionGridSpec, Placement, Product,
};
pub use error::{DesignError, GenerateError, GeometryError};
pub use fit::{Clash, FitColor, ProductFit};
pub use geometry::{
    Area, Bounds, CURVE_TOLERANCE_MM, Line, Path, Point, Polygon, Region, Segment, Transform,
};
pub use insert::{FINGER_NOTCH_RADIUS_MM, Insert, Mount, Stance};
pub use material::{Board, Material, MmRange};
pub use outer_box::OuterBox;
pub use packaging::Packaging;
pub use piece::{FoldDirection, Hinge, Panel, Piece};
pub use preview::{PREVIEW_GRID_MM, PREVIEW_TOLERANCE_MM};
