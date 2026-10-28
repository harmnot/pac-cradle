//! `Design::validate` accepts real designs and names the field behind each problem.

mod common;

use insert_core::limits::MAX_PRODUCTS;
use insert_core::{BoxSize, DesignError, InsertSpec, Placement, Point, Polygon};

fn length_field(error: Result<(), DesignError>) -> String {
    match error {
        Err(DesignError::LengthOutOfRange { field, .. }) => field,
        other => panic!("expected LengthOutOfRange, got {other:?}"),
    }
}

#[test]
fn sample_designs_are_valid() {
    assert_eq!(common::grid_design().validate(), Ok(()));
    assert_eq!(common::card_design().validate(), Ok(()));
    assert_eq!(common::cradle_design().validate(), Ok(()));
}

#[test]
fn board_thickness_must_be_above_zero() {
    let mut design = common::grid_design();
    design.material.thickness_mm = 0.0;
    assert_eq!(length_field(design.validate()), "material.thickness_mm");
}

#[test]
fn clearance_can_be_zero_but_not_negative() {
    let mut design = common::grid_design();
    design.material.clearance_mm = 0.0;
    assert_eq!(design.validate(), Ok(()));

    design.material.clearance_mm = -0.5;
    assert_eq!(length_field(design.validate()), "material.clearance_mm");
}

#[test]
fn heights_must_be_numbers() {
    let mut design = common::card_design();
    design.products[1].height_mm = f64::NAN;
    assert_eq!(length_field(design.validate()), "products[1].height_mm");
}

#[test]
fn box_material_and_fixed_size_are_checked() {
    let mut design = common::card_design();
    if let Some(outer_box) = design.outer_box.as_mut() {
        outer_box.inside_mm = Some(BoxSize {
            length_mm: 160.0,
            width_mm: 0.0,
            height_mm: 40.0,
        });
    }
    assert_eq!(
        length_field(design.validate()),
        "outer_box.inside_mm.width_mm"
    );

    let mut design = common::cradle_design();
    if let Some(outer_box) = design.outer_box.as_mut() {
        outer_box.material.thickness_mm = 25.0;
    }
    assert_eq!(
        length_field(design.validate()),
        "outer_box.material.thickness_mm"
    );
}

#[test]
fn product_rules() {
    let mut design = common::card_design();
    design.products.clear();
    assert_eq!(design.validate(), Err(DesignError::NoProducts));

    let mut design = common::card_design();
    design.products = (0..=MAX_PRODUCTS)
        .map(|index| common::product(&format!("p{index}"), common::rectangle(10.0, 10.0)))
        .collect();
    assert!(matches!(
        design.validate(),
        Err(DesignError::TooManyProducts { .. })
    ));

    let mut design = common::card_design();
    design.products[1].id = "gloss".to_owned();
    assert_eq!(
        design.validate(),
        Err(DesignError::DuplicateProductId("gloss".to_owned()))
    );

    let mut design = common::card_design();
    design.products[0].id = "  ".to_owned();
    assert_eq!(design.validate(), Err(DesignError::EmptyProductId));

    let mut design = common::card_design();
    design.products[0].quantity = 0;
    assert!(matches!(
        design.validate(),
        Err(DesignError::CountOutOfRange { field, .. }) if field == "products[0].quantity"
    ));
}

#[test]
fn outlines_must_enclose_an_area() {
    let mut design = common::cradle_design();
    design.products[0].contour_mm = Polygon(vec![Point::new(0.0, 0.0), Point::new(10.0, 0.0)]);
    assert!(matches!(
        design.validate(),
        Err(DesignError::ContourPointCount { points: 2, .. })
    ));

    design.products[0].contour_mm = Polygon(vec![
        Point::new(0.0, 0.0),
        Point::new(10.0, 0.0),
        Point::new(20.0, 0.0),
    ]);
    assert!(matches!(
        design.validate(),
        Err(DesignError::ContourTooSmall { .. })
    ));

    design.products[0].contour_mm = Polygon(vec![
        Point::new(0.0, 0.0),
        Point::new(1.0e9, 0.0),
        Point::new(0.0, 10.0),
    ]);
    assert!(matches!(
        design.validate(),
        Err(DesignError::ContourOutOfRange { .. })
    ));
}

#[test]
fn a_partition_grid_holds_one_product_type_and_enough_cells() {
    let mut design = common::grid_design();
    design
        .products
        .push(common::product("lid", common::circle(40.0)));
    assert_eq!(
        design.validate(),
        Err(DesignError::GridNeedsOneProduct { count: 2 })
    );

    let mut design = common::grid_design();
    design.products[0].quantity = 7;
    assert_eq!(
        design.validate(),
        Err(DesignError::GridTooSmall {
            rows: 2,
            columns: 3,
            quantity: 7
        })
    );

    let mut design = common::grid_design();
    if let InsertSpec::PartitionGrid(grid) = &mut design.insert {
        grid.rows = 0;
    }
    assert!(matches!(
        design.validate(),
        Err(DesignError::CountOutOfRange { field, .. }) if field == "insert.rows"
    ));
}

#[test]
fn placements_must_match_the_products() {
    let place = |product_id: &str| Placement {
        product_id: product_id.to_owned(),
        center_mm: Point::new(40.0, 40.0),
        rotation_deg: 0.0,
    };
    let with_placements = |placements: Vec<Placement>| {
        let mut design = common::card_design();
        if let InsertSpec::CutoutCard(card) = &mut design.insert {
            card.placements = placements;
        }
        design
    };

    assert_eq!(
        with_placements(vec![place("ghost")]).validate(),
        Err(DesignError::UnknownPlacementProduct("ghost".to_owned()))
    );
    assert_eq!(
        with_placements(vec![place("gloss"), place("gloss")]).validate(),
        Err(DesignError::TooManyPlacements {
            product_id: "gloss".to_owned(),
            quantity: 1
        })
    );

    let mut spinning = place("compact");
    spinning.rotation_deg = f64::INFINITY;
    assert!(matches!(
        with_placements(vec![spinning]).validate(),
        Err(DesignError::PlacementOutOfRange { .. })
    ));
}
