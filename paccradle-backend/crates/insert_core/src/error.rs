//! Why a design can't be generated.
//!
//! Fields are named by their JSON path (e.g. `products[0].height_mm`) so the API and
//! the editor can point at the value to fix. The API maps these to error codes (M1-D6).

use thiserror::Error;

/// A design that failed [`Design::validate`](crate::Design::validate).
#[derive(Debug, Clone, PartialEq, Error)]
pub enum DesignError {
    #[error("`{field}` is {value} mm; allowed: {allowed}")]
    LengthOutOfRange {
        field: String,
        value: f64,
        allowed: String,
    },
    #[error("`{field}` is {value}; allowed: {min} to {max}")]
    CountOutOfRange {
        field: String,
        value: u32,
        min: u32,
        max: u32,
    },
    #[error("a design needs at least one product")]
    NoProducts,
    #[error("a design holds at most {max} product types, got {count}")]
    TooManyProducts { count: usize, max: usize },
    #[error("product ids can't be empty")]
    EmptyProductId,
    #[error("product id `{0}` is used twice")]
    DuplicateProductId(String),
    #[error("the outline of product `{product_id}` has {points} points; it needs {min} to {max}")]
    ContourPointCount {
        product_id: String,
        points: usize,
        min: usize,
        max: usize,
    },
    #[error(
        "the outline of product `{product_id}` has a point that isn't a number or is too far out"
    )]
    ContourOutOfRange { product_id: String },
    #[error("the outline of product `{product_id}` encloses less than {min_area_mm2} mm²")]
    ContourTooSmall {
        product_id: String,
        min_area_mm2: f64,
    },
    #[error("a partition grid holds one product type, got {count}")]
    GridNeedsOneProduct { count: usize },
    #[error("a {rows} × {columns} grid is too small for {quantity} products")]
    GridTooSmall {
        rows: u32,
        columns: u32,
        quantity: u32,
    },
    #[error("a placement refers to unknown product `{0}`")]
    UnknownPlacementProduct(String),
    #[error("product `{product_id}` has more placements than its quantity of {quantity}")]
    TooManyPlacements { product_id: String, quantity: u32 },
    #[error(
        "a placement of product `{product_id}` has a position or turn that isn't a number or is too far out"
    )]
    PlacementOutOfRange { product_id: String },
}
