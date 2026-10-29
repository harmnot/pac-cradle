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
    #[error("`{field}` is {value}; allowed: 0 to 1")]
    FractionOutOfRange { field: String, value: f64 },
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

/// Why an insert couldn't be generated.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum GenerateError {
    #[error(transparent)]
    Invalid(#[from] DesignError),
    #[error(transparent)]
    Geometry(#[from] GeometryError),
    #[error(
        "the products need about {needed_length_mm:.1} × {needed_width_mm:.1} mm, but the box leaves {room_length_mm:.1} × {room_width_mm:.1} mm"
    )]
    DoesNotFit {
        needed_length_mm: f64,
        needed_width_mm: f64,
        room_length_mm: f64,
        room_width_mm: f64,
    },
    #[error("the cuts split the {piece} into {parts} parts; it must stay in one piece")]
    FallsApart { piece: &'static str, parts: usize },
    #[error("a 1 × 1 grid without a frame has no dividers to cut")]
    EmptyGrid,
    #[error(
        "a cradle deck of this board needs at least {min_length_mm:.1} mm of length and {min_height_mm:.1} mm of height for its legs and lock tabs"
    )]
    DeckTooSmall {
        min_length_mm: f64,
        min_height_mm: f64,
    },
    #[error(
        "the products need {needed_height_mm:.1} mm of height, but the box leaves {room_height_mm:.1} mm"
    )]
    TooTallForBox {
        needed_height_mm: f64,
        room_height_mm: f64,
    },
    #[error(
        "this box style needs at least {min_length_mm:.1} × {min_width_mm:.1} × {min_height_mm:.1} mm inside in this board"
    )]
    BoxTooSmall {
        min_length_mm: f64,
        min_width_mm: f64,
        min_height_mm: f64,
    },
}

/// A geometry helper that couldn't run.
///
/// Validated designs stay far inside these bounds, so generators treat this as an
/// internal error, not as something the user can fix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum GeometryError {
    #[error("a coordinate or distance isn't a number, or the shapes span more than about 100 m")]
    OutOfRange,
}
