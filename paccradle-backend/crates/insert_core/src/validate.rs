//! Checks a user's design before anything is generated. Returns the first problem found.

use std::collections::{HashMap, HashSet};

use crate::design::{BoxSpec, Design, InsertSpec, PartitionGridSpec, Placement, Product};
use crate::error::DesignError;
use crate::geometry::Point;
use crate::limits::{
    MAX_CONTOUR_POINTS, MAX_GAP_MM, MAX_GRID_LINES, MAX_LENGTH_MM, MAX_PRODUCTS, MAX_QUANTITY,
    MAX_THICKNESS_MM, MIN_CONTOUR_AREA_MM2, MIN_CONTOUR_POINTS,
};
use crate::material::Material;

impl Design {
    /// Checks every value against [`crate::limits`] and the rules of the insert style.
    pub fn validate(&self) -> Result<(), DesignError> {
        validate_material("material", &self.material)?;
        validate_products(&self.products)?;
        validate_insert(self)?;
        match &self.outer_box {
            Some(outer_box) => validate_box(outer_box),
            None => Ok(()),
        }
    }
}

/// Allowed range for a length in mm.
#[derive(Clone, Copy)]
struct Limits {
    min: f64,
    max: f64,
    /// Whether `min` itself is allowed.
    min_allowed: bool,
}

impl Limits {
    /// Sizes and heights: above 0.
    const LENGTH: Self = Self {
        min: 0.0,
        max: MAX_LENGTH_MM,
        min_allowed: false,
    };
    /// Margins: 0 or more.
    const SPACING: Self = Self {
        min: 0.0,
        max: MAX_LENGTH_MM,
        min_allowed: true,
    };
    /// Clearances and slot tolerances: 0 or more.
    const GAP: Self = Self {
        min: 0.0,
        max: MAX_GAP_MM,
        min_allowed: true,
    };
    /// Board thickness: above 0.
    const THICKNESS: Self = Self {
        min: 0.0,
        max: MAX_THICKNESS_MM,
        min_allowed: false,
    };

    fn check(self, field: &str, value: f64) -> Result<(), DesignError> {
        let above_min = match self.min_allowed {
            true => value >= self.min,
            false => value > self.min,
        };
        if value.is_finite() && above_min && value <= self.max {
            return Ok(());
        }
        Err(DesignError::LengthOutOfRange {
            field: field.to_owned(),
            value,
            allowed: self.describe(),
        })
    }

    fn describe(self) -> String {
        let lower = match self.min_allowed {
            true => "at least",
            false => "more than",
        };
        format!("{lower} {} mm and at most {} mm", self.min, self.max)
    }
}

/// Allowed range for a count.
#[derive(Clone, Copy)]
struct CountLimits {
    min: u32,
    max: u32,
}

impl CountLimits {
    fn check(self, field: &str, value: u32) -> Result<(), DesignError> {
        if (self.min..=self.max).contains(&value) {
            return Ok(());
        }
        Err(DesignError::CountOutOfRange {
            field: field.to_owned(),
            value,
            min: self.min,
            max: self.max,
        })
    }
}

/// True when the point is a real number within reach of the origin.
fn within_reach(point: Point) -> bool {
    point.is_finite() && point.x.abs() <= MAX_LENGTH_MM && point.y.abs() <= MAX_LENGTH_MM
}

fn validate_material(prefix: &str, material: &Material) -> Result<(), DesignError> {
    Limits::THICKNESS.check(&format!("{prefix}.thickness_mm"), material.thickness_mm)?;
    Limits::GAP.check(&format!("{prefix}.clearance_mm"), material.clearance_mm)?;
    Limits::GAP.check(
        &format!("{prefix}.slot_tolerance_mm"),
        material.slot_tolerance_mm,
    )
}

fn validate_products(products: &[Product]) -> Result<(), DesignError> {
    if products.is_empty() {
        return Err(DesignError::NoProducts);
    }
    if products.len() > MAX_PRODUCTS {
        return Err(DesignError::TooManyProducts {
            count: products.len(),
            max: MAX_PRODUCTS,
        });
    }
    let mut seen_ids = HashSet::new();
    for (index, product) in products.iter().enumerate() {
        validate_product(index, product)?;
        if !seen_ids.insert(product.id.as_str()) {
            return Err(DesignError::DuplicateProductId(product.id.clone()));
        }
    }
    Ok(())
}

fn validate_product(index: usize, product: &Product) -> Result<(), DesignError> {
    if product.id.trim().is_empty() {
        return Err(DesignError::EmptyProductId);
    }
    let points = product.contour_mm.points();
    if !(MIN_CONTOUR_POINTS..=MAX_CONTOUR_POINTS).contains(&points.len()) {
        return Err(DesignError::ContourPointCount {
            product_id: product.id.clone(),
            points: points.len(),
            min: MIN_CONTOUR_POINTS,
            max: MAX_CONTOUR_POINTS,
        });
    }
    if !points.iter().all(|&point| within_reach(point)) {
        return Err(DesignError::ContourOutOfRange {
            product_id: product.id.clone(),
        });
    }
    if product.contour_mm.signed_area_mm2().abs() < MIN_CONTOUR_AREA_MM2 {
        return Err(DesignError::ContourTooSmall {
            product_id: product.id.clone(),
            min_area_mm2: MIN_CONTOUR_AREA_MM2,
        });
    }
    Limits::LENGTH.check(&format!("products[{index}].height_mm"), product.height_mm)?;
    CountLimits {
        min: 1,
        max: MAX_QUANTITY,
    }
    .check(&format!("products[{index}].quantity"), product.quantity)
}

fn validate_insert(design: &Design) -> Result<(), DesignError> {
    match &design.insert {
        InsertSpec::PartitionGrid(grid) => validate_grid(grid, &design.products),
        InsertSpec::CutoutCard(card) => {
            Limits::SPACING.check("insert.margin_mm", card.margin_mm)?;
            validate_placements(&card.placements, &design.products)
        }
        InsertSpec::CradlePlatform(cradle) => {
            Limits::LENGTH.check("insert.deck_height_mm", cradle.deck_height_mm)?;
            Limits::SPACING.check("insert.margin_mm", cradle.margin_mm)?;
            validate_placements(&cradle.placements, &design.products)
        }
    }
}

fn validate_grid(grid: &PartitionGridSpec, products: &[Product]) -> Result<(), DesignError> {
    let [product] = products else {
        return Err(DesignError::GridNeedsOneProduct {
            count: products.len(),
        });
    };
    let lines = CountLimits {
        min: 1,
        max: MAX_GRID_LINES,
    };
    lines.check("insert.rows", grid.rows)?;
    lines.check("insert.columns", grid.columns)?;
    Limits::LENGTH.check("insert.divider_height_mm", grid.divider_height_mm)?;
    if grid.rows * grid.columns < product.quantity {
        return Err(DesignError::GridTooSmall {
            rows: grid.rows,
            columns: grid.columns,
            quantity: product.quantity,
        });
    }
    Ok(())
}

fn validate_placements(placements: &[Placement], products: &[Product]) -> Result<(), DesignError> {
    let mut placed: HashMap<&str, u32> = HashMap::new();
    for placement in placements {
        let Some(product) = products
            .iter()
            .find(|product| product.id == placement.product_id)
        else {
            return Err(DesignError::UnknownPlacementProduct(
                placement.product_id.clone(),
            ));
        };
        if !within_reach(placement.center_mm) || !placement.rotation_deg.is_finite() {
            return Err(DesignError::PlacementOutOfRange {
                product_id: product.id.clone(),
            });
        }
        let count = placed.entry(product.id.as_str()).or_insert(0);
        *count += 1;
        if *count > product.quantity {
            return Err(DesignError::TooManyPlacements {
                product_id: product.id.clone(),
                quantity: product.quantity,
            });
        }
    }
    Ok(())
}

fn validate_box(outer_box: &BoxSpec) -> Result<(), DesignError> {
    validate_material("outer_box.material", &outer_box.material)?;
    let Some(inside) = outer_box.inside_mm else {
        return Ok(());
    };
    Limits::LENGTH.check("outer_box.inside_mm.length_mm", inside.length_mm)?;
    Limits::LENGTH.check("outer_box.inside_mm.width_mm", inside.width_mm)?;
    Limits::LENGTH.check("outer_box.inside_mm.height_mm", inside.height_mm)
}
