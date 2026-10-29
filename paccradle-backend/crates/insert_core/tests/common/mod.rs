//! Sample designs and shapes shared by the integration tests.
#![allow(dead_code)]

pub mod fold;
pub mod golden;
pub mod measure;
pub mod pdf;
pub mod solid;
pub mod svg;

use insert_core::{
    Align, Area, Board, BoxSize, BoxSpec, BoxStyle, CradlePlatformSpec, CutoutCardSpec, Design,
    FingerNotches, Insert, InsertSpec, Material, OuterBox, Packaging, Panel, PartitionGridSpec,
    Path, Piece, Placement, Point, Polygon, Product, Segment, Transform,
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
            finger_notches: Some(FingerNotches::default()),
            align: Align::Center,
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
            align: Align::Center,
            placements: Vec::new(),
        }),
        outer_box: Some(BoxSpec {
            style: BoxStyle::Mailer,
            material: Material::starting_values(Board::EFlute),
            inside_mm: None,
        }),
    }
}

/// Where a placement puts the product's outline: turned about the centre of its
/// bounding box, which then lands on `center_mm`.
pub fn placed_outline(product: &Product, placement: &Placement) -> Area {
    let pivot = product
        .contour_mm
        .bounds()
        .unwrap_or_else(|| panic!("product `{}` has no outline", product.id))
        .center();
    let place = Transform::shift(Point::ORIGIN - pivot).then(Transform {
        turn_deg: placement.rotation_deg,
        shift: placement.center_mm,
    });
    Area::from_polygon(&product.contour_mm.transformed(place))
        .unwrap_or_else(|error| panic!("outline of `{}`: {error}", product.id))
}

/// A packaging built by hand for the exports: a Ø60 disc (a full-circle arc, centre
/// (30, 30)) with a quarter-turn crease arc and a perforation, cut twice, as the insert
/// and again as a one-piece "box". Checks arcs, mirroring and the perforation layer.
pub fn arc_packaging() -> Packaging {
    let circle = Path {
        start: Point::new(60.0, 30.0),
        segments: vec![Segment::Arc {
            center: Point::new(30.0, 30.0),
            sweep_deg: 360.0,
        }],
        closed: true,
    };
    // A quarter turn, clockwise on screen, from the top of the circle to its right.
    let crease = Path {
        start: Point::new(30.0, 10.0),
        segments: vec![Segment::Arc {
            center: Point::new(30.0, 30.0),
            sweep_deg: 90.0,
        }],
        closed: false,
    };
    let perforation = Path {
        start: Point::new(15.0, 45.0),
        segments: vec![Segment::Line {
            to: Point::new(25.0, 45.0),
        }],
        closed: false,
    };
    let piece = Piece {
        name: "disc".to_owned(),
        quantity: 2,
        outline: circle,
        holes: Vec::new(),
        creases: vec![crease],
        perforations: vec![perforation],
        panels: vec![Panel {
            outline: Polygon(Vec::new()),
            holes: Vec::new(),
        }],
        hinges: Vec::new(),
    };
    let size = BoxSize {
        length_mm: 60.0,
        width_mm: 60.0,
        height_mm: 10.0,
    };
    Packaging {
        insert: Insert {
            pieces: vec![piece.clone()],
            length_mm: 60.0,
            width_mm: 60.0,
            height_mm: 1.0,
            placements: Vec::new(),
            mounts: Vec::new(),
        },
        outer_box: Some(OuterBox {
            style: BoxStyle::Mailer,
            pieces: vec![piece],
            inside_mm: size,
            outside_mm: size,
            mounts: Vec::new(),
        }),
    }
}
