//! M1-A2 "done when": offsets of a rectangle, a circle and concave shapes land within
//! 0.01 mm of the exact offset. Booleans and the area rules are checked here too.

mod common;

use common::measure::{
    boundary_points, check_offset, circle_path, dumbbell, has_corner_near, l_shape, rectangle_at,
};
use insert_core::{Area, CURVE_TOLERANCE_MM, GeometryError, Point, Polygon};

/// The M1-A2 accuracy bar.
const ACCURACY_MM: f64 = 0.01;

fn square(corner: [f64; 2], side_mm: f64) -> Area {
    match Area::from_polygon(&rectangle_at(Point::from(corner), (side_mm, side_mm))) {
        Ok(area) => area,
        Err(error) => panic!("square at {corner:?}: {error}"),
    }
}

#[test]
fn rectangle_grows_with_round_corners() {
    let rectangle = rectangle_at(Point::new(0.0, 0.0), (60.0, 40.0));
    let check = check_offset(&rectangle, 1.5);

    assert!(
        check.worst_error_mm <= ACCURACY_MM,
        "{}",
        check.worst_error_mm
    );
    assert_eq!(check.wrong_side, 0);
    let [region] = check.result.regions() else {
        panic!("expected one region");
    };
    assert!(region.holes.is_empty());
    // 63 × 43 with each corner a quarter circle of radius 1.5 instead of a square.
    let exact = 63.0 * 43.0 - (4.0 - std::f64::consts::PI) * 1.5 * 1.5;
    assert!((check.result.area_mm2() - exact).abs() < 0.05);
}

#[test]
fn rectangle_shrinks_with_sharp_corners() {
    let rectangle = rectangle_at(Point::new(0.0, 0.0), (60.0, 40.0));
    let check = check_offset(&rectangle, -1.5);

    assert!(
        check.worst_error_mm <= ACCURACY_MM,
        "{}",
        check.worst_error_mm
    );
    assert_eq!(check.wrong_side, 0);
    let [region] = check.result.regions() else {
        panic!("expected one region");
    };
    assert_eq!(region.outer.points().len(), 4);
    for corner in [[1.5, 1.5], [58.5, 1.5], [58.5, 38.5], [1.5, 38.5]] {
        assert!(has_corner_near(
            &check.result,
            (Point::from(corner), ACCURACY_MM)
        ));
    }
}

#[test]
fn circle_grows_and_shrinks_within_accuracy() {
    let radius = 26.0;
    let circle = Polygon(circle_path(radius).flatten(CURVE_TOLERANCE_MM));
    for distance in [1.5, -1.5, 10.0, -3.0, 0.3] {
        let check = check_offset(&circle, distance);
        assert_eq!(check.result.regions().len(), 1);
        // Measured against the true circle, so arc flattening counts against the bar too.
        let origin = Point::new(0.0, 0.0);
        for point in boundary_points(&check.result) {
            let error = (point.distance_to(origin) - (radius + distance)).abs();
            assert!(error <= ACCURACY_MM, "offset {distance}: off by {error} mm");
        }
    }
}

#[test]
fn concave_shape_offsets_within_accuracy() {
    for distance in [2.0, -2.0, 1.0, -1.0, 8.0] {
        let check = check_offset(&l_shape(), distance);
        assert!(
            check.worst_error_mm <= ACCURACY_MM,
            "offset {distance}: off by {}",
            check.worst_error_mm
        );
        assert_eq!(check.wrong_side, 0, "offset {distance}");
        assert_eq!(check.result.regions().len(), 1, "offset {distance}");
    }
}

#[test]
fn inside_corner_stays_sharp_when_growing() {
    // The L's inside corner at (25, 20) moves to (27, 22) and keeps its point.
    let check = check_offset(&l_shape(), 2.0);
    assert!(has_corner_near(
        &check.result,
        (Point::new(27.0, 22.0), ACCURACY_MM)
    ));
}

#[test]
fn shrinking_splits_a_narrow_neck() {
    let grown = check_offset(&dumbbell(), 1.5);
    assert_eq!(grown.result.regions().len(), 1);

    // The neck is 4 mm wide, so shrinking by 3 leaves two separate squares.
    let shrunk = check_offset(&dumbbell(), -3.0);
    assert_eq!(shrunk.result.regions().len(), 2);
    assert!(
        shrunk.worst_error_mm <= ACCURACY_MM,
        "{}",
        shrunk.worst_error_mm
    );
    assert_eq!(shrunk.wrong_side, 0);
}

#[test]
fn shrinking_past_the_middle_leaves_nothing() {
    let shrunk = square([0.0, 0.0], 10.0).offset(-6.0).unwrap();
    assert!(shrunk.is_empty());
    assert_eq!(shrunk.area_mm2(), 0.0);
}

#[test]
fn growing_a_frame_shrinks_its_hole() {
    let frame = square([0.0, 0.0], 50.0)
        .difference(&square([10.0, 10.0], 30.0))
        .unwrap();
    let grown = frame.offset(2.0).unwrap();

    let [region] = grown.regions() else {
        panic!("expected one region");
    };
    let [hole] = region.holes.as_slice() else {
        panic!("expected one hole");
    };
    // The hole's corners point into the frame, so they stay sharp: an exact 26 mm square.
    assert!((hole.signed_area_mm2().abs() - 26.0 * 26.0).abs() < ACCURACY_MM);
    assert!(has_corner_near(
        &grown,
        (Point::new(12.0, 12.0), ACCURACY_MM)
    ));
}

#[test]
fn zero_offset_changes_nothing() {
    let area = Area::from_polygon(&l_shape()).unwrap();
    assert_eq!(area.offset(0.0).unwrap(), area);
}

#[test]
fn offset_rejects_distances_that_are_not_numbers() {
    let area = square([0.0, 0.0], 10.0);
    assert_eq!(area.offset(f64::NAN), Err(GeometryError::OutOfRange));
    assert_eq!(area.offset(f64::INFINITY), Err(GeometryError::OutOfRange));
}

#[test]
fn union_joins_overlapping_squares() {
    let joined = square([0.0, 0.0], 10.0)
        .union(&square([5.0, 5.0], 10.0))
        .unwrap();
    assert_eq!(joined.regions().len(), 1);
    assert!((joined.area_mm2() - 175.0).abs() < 1e-6);
}

#[test]
fn intersection_keeps_the_overlap() {
    let overlap = square([0.0, 0.0], 10.0)
        .intersection(&square([5.0, 5.0], 10.0))
        .unwrap();
    assert_eq!(overlap.regions().len(), 1);
    assert!((overlap.area_mm2() - 25.0).abs() < 1e-6);
    assert!(has_corner_near(&overlap, (Point::new(5.0, 5.0), 1e-6)));
    assert!(has_corner_near(&overlap, (Point::new(10.0, 10.0), 1e-6)));
}

#[test]
fn difference_can_cut_a_hole() {
    let card = square([0.0, 0.0], 10.0)
        .difference(&square([3.0, 3.0], 4.0))
        .unwrap();
    let [region] = card.regions() else {
        panic!("expected one region");
    };
    assert_eq!(region.holes.len(), 1);
    assert!((card.area_mm2() - 84.0).abs() < 1e-6);
}

#[test]
fn difference_of_separate_shapes_keeps_the_first() {
    let first = square([0.0, 0.0], 10.0);
    let kept = first.difference(&square([20.0, 0.0], 10.0)).unwrap();
    assert!((kept.area_mm2() - 100.0).abs() < 1e-6);
}

#[test]
fn rings_always_come_out_the_same_way_round() {
    let clockwise = rectangle_at(Point::new(0.0, 0.0), (10.0, 10.0));
    let mut anticlockwise = clockwise.clone();
    anticlockwise.0.reverse();
    let from_clockwise = Area::from_polygon(&clockwise).unwrap();
    assert_eq!(Area::from_polygon(&anticlockwise).unwrap(), from_clockwise);

    let card = from_clockwise.difference(&square([3.0, 3.0], 4.0)).unwrap();
    for region in card.regions() {
        assert!(
            region.outer.signed_area_mm2() > 0.0,
            "outer runs clockwise on screen"
        );
        for hole in &region.holes {
            assert!(hole.signed_area_mm2() < 0.0, "holes run the other way");
        }
    }
}

#[test]
fn outline_that_crosses_itself_keeps_both_loops() {
    // A bow tie: two triangles meeting at (5, 5).
    let bow_tie = Polygon(
        [[0.0, 0.0], [10.0, 10.0], [10.0, 0.0], [0.0, 10.0]]
            .map(Point::from)
            .to_vec(),
    );
    let area = Area::from_polygon(&bow_tie).unwrap();
    assert_eq!(area.regions().len(), 2);
    assert!((area.area_mm2() - 50.0).abs() < 1e-6);
}

#[test]
fn from_polygons_joins_outlines_whichever_way_they_run() {
    let first = rectangle_at(Point::new(0.0, 0.0), (10.0, 10.0));
    let mut second = rectangle_at(Point::new(5.0, 5.0), (10.0, 10.0));
    second.0.reverse();
    let joined = Area::from_polygons(&[first, second]).unwrap();
    assert_eq!(joined.regions().len(), 1);
    assert!((joined.area_mm2() - 175.0).abs() < 1e-6);
}

#[test]
fn coordinates_out_of_range_are_rejected() {
    let not_a_number = Polygon(vec![
        Point::new(0.0, 0.0),
        Point::new(f64::NAN, 0.0),
        Point::new(10.0, 10.0),
    ]);
    assert_eq!(
        Area::from_polygon(&not_a_number),
        Err(GeometryError::OutOfRange)
    );

    // The grid moves with the shape: far from the origin is fine, too big isn't.
    let far_away = rectangle_at(Point::new(1_000_000.0, 0.0), (10.0, 10.0));
    assert!(Area::from_polygon(&far_away).is_ok());
    let too_big = rectangle_at(Point::new(0.0, 0.0), (200_000.0, 10.0));
    assert_eq!(Area::from_polygon(&too_big), Err(GeometryError::OutOfRange));
    assert_eq!(
        square([0.0, 0.0], 10.0).offset(1_000_000.0),
        Err(GeometryError::OutOfRange)
    );
}

#[test]
fn shapes_built_apart_still_meet_where_they_share_an_edge() {
    // A row of strips at odd positions, each built on its own the way a box's panels
    // are, then joined. Each strip's edges must snap where its neighbour's do, or the
    // row splits with a 0.1 µm gap (a 118.953 mm shipping box once fell into 5 parts).
    let edges: Vec<f64> = (0..=60)
        .map(|step| {
            let step = f64::from(step);
            3.7 * step + 0.000_123_4 * step * step
        })
        .collect();
    let strips: Vec<Area> = edges
        .windows(2)
        .map(|pair| {
            let (left, right) = (pair[0], pair[1]);
            let strip = Polygon(vec![
                Point::new(left, 0.0),
                Point::new(right, 0.0),
                Point::new(right, 10.0),
                Point::new(left, 10.0),
            ]);
            Area::from_polygon(&strip).unwrap()
        })
        .collect();
    let row = Area::union_all(&strips).unwrap();
    assert_eq!(row.regions().len(), 1);
    assert!((row.area_mm2() - 10.0 * edges[60]).abs() < 1e-3);
}
