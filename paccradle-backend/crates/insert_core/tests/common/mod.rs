//! Sample designs shared by the integration tests.
#![allow(dead_code)]

use insert_core::{
    Board, BoxSize, BoxSpec, BoxStyle, CradlePlatformSpec, CutoutCardSpec, Design, InsertSpec,
    Material, PartitionGridSpec, Placement, Point, Polygon, Product,
};

/// Axis-aligned rectangle from the origin, like a vision outline of a box.
pub fn rectangle(length_mm: f64, width_mm: f64) -> Polygon {
    Polygon(vec![
        Point::new(0.0, 0.0),
        Point::new(length_mm, 0.0),
        Point::new(length_mm, width_mm),
        Point::new(0.0, width_mm),
    ])
}

/// Circle outline with 64 points, inside its bounding box from the origin.
pub fn circle(diameter_mm: f64) -> Polygon {
    let radius = diameter_mm / 2.0;
    let points = (0..64)
        .map(|step| {
            let angle = f64::from(step) * std::f64::consts::TAU / 64.0;
            Point::new(radius + radius * angle.cos(), radius + radius * angle.sin())
        })
        .collect();
    Polygon(points)
}

/// A product with a typed height.
pub fn product(id: &str, contour_mm: Polygon) -> Product {
    Product {
        id: id.to_owned(),
        contour_mm,
        height_mm: 95.0,
        quantity: 1,
    }
}

/// Six candle jars in a 2 × 3 partition grid of E-flute, no box.
pub fn grid_design() -> Design {
    let mut jar = product("jar", circle(72.0));
    jar.quantity = 6;
    Design {
        material: Material::starting_values(Board::EFlute),
        products: vec![jar],
        insert: InsertSpec::PartitionGrid(PartitionGridSpec {
            rows: 2,
            columns: 3,
            divider_height_mm: 60.0,
            outer_frame: false,
            finger_notches: true,
        }),
        outer_box: None,
    }
}

/// A lip gloss and a compact on one paperboard card, in a shipping box of fixed size.
pub fn card_design() -> Design {
    Design {
        material: Material::starting_values(Board::Paperboard),
        products: vec![
            product("gloss", rectangle(18.0, 90.0)),
            product("compact", circle(60.0)),
        ],
        insert: InsertSpec::CutoutCard(CutoutCardSpec {
            margin_mm: 10.0,
            finger_notches: true,
            placements: vec![Placement {
                product_id: "gloss".to_owned(),
                center_mm: Point::new(30.0, 60.0),
                rotation_deg: 90.0,
            }],
        }),
        outer_box: Some(BoxSpec {
            style: BoxStyle::Shipping,
            material: Material::starting_values(Board::BFlute),
            inside_mm: Some(BoxSize {
                length_mm: 160.0,
                width_mm: 120.0,
                height_mm: 40.0,
            }),
        }),
    }
}

/// One perfume bottle on a chipboard cradle, auto-laid out, in a mailer box.
pub fn cradle_design() -> Design {
    Design {
        material: Material::starting_values(Board::Chipboard),
        products: vec![product("perfume", rectangle(52.0, 38.0))],
        insert: InsertSpec::CradlePlatform(CradlePlatformSpec {
            deck_height_mm: 40.0,
            margin_mm: 12.0,
            placements: Vec::new(),
        }),
        outer_box: Some(BoxSpec {
            style: BoxStyle::Mailer,
            material: Material::starting_values(Board::EFlute),
            inside_mm: None,
        }),
    }
}
