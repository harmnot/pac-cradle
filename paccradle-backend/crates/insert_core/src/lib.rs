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

pub mod design;
pub mod error;
pub mod geometry;
pub mod limits;
pub mod material;
pub mod piece;
mod validate;

pub use design::{
    BoxSize, BoxSpec, BoxStyle, CradlePlatformSpec, CutoutCardSpec, Design, InsertSpec,
    PartitionGridSpec, Placement, Product,
};
pub use error::DesignError;
pub use geometry::{Line, Path, Point, Polygon, Segment};
pub use material::{Board, Material, MmRange};
pub use piece::{FoldDirection, Hinge, Panel, Piece};
