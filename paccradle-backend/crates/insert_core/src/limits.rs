//! Bounds that [`Design::validate`](crate::Design::validate) enforces.
//!
//! Designs come from users, so these stop nonsense values and jobs too big to generate.
//! They sit well above any real insert; the manufacturability checks (M1-A7) warn much earlier.

/// Longest length accepted: larger than any sheet a printer or cutter takes.
pub const MAX_LENGTH_MM: f64 = 2000.0;
/// Thickest board accepted. Double-wall corrugated is about 7 mm.
pub const MAX_THICKNESS_MM: f64 = 10.0;
/// Largest clearance or slot tolerance accepted, per side.
pub const MAX_GAP_MM: f64 = 10.0;
/// Most product types in one design.
pub const MAX_PRODUCTS: usize = 32;
/// Fewest points that can enclose an area.
pub const MIN_CONTOUR_POINTS: usize = 3;
/// Most points in one outline. Vision simplifies at 0.2 mm, so real outlines stay far below this.
pub const MAX_CONTOUR_POINTS: usize = 10_000;
/// Smallest outline area accepted.
pub const MIN_CONTOUR_AREA_MM2: f64 = 1.0;
/// Most copies of one product.
pub const MAX_QUANTITY: u32 = 1_000;
/// Most rows, or columns, in a partition grid.
pub const MAX_GRID_LINES: u32 = 50;
