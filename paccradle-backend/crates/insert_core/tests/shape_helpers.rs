//! Helpers added for the generators (M1-A3): bounds, transforms, hulls and straightening.

mod common;

use common::measure::rectangle_at;
use insert_core::{Area, Bounds, CURVE_TOLERANCE_MM, Path, Point, Polygon, Transform};

fn polygon(points: &[[f64; 2]]) -> Polygon {
    Polygon(points.iter().copied().map(Point::from).collect())
}

fn close(a: Point, b: Point) -> bool {
    a.distance_to(b) < 1e-9
}

#[test]
fn transform_turns_clockwise_on_screen_then_shifts() {
    let quarter = Transform {
        turn_deg: 90.0,
        shift: Point::new(5.0, 0.0),
    };
    // +x turns towards +y (down on screen), then moves 5 right.
    assert!(close(
        quarter.apply(Point::new(1.0, 0.0)),
        Point::new(5.0, 1.0)
    ));
}

#[test]
fn transforms_combine_in_order() {
    let first = Transform {
        turn_deg: 30.0,
        shift: Point::new(2.0, -1.0),
    };
    let second = Transform {
        turn_deg: -75.0,
        shift: Point::new(-4.0, 3.0),
    };
    let point = Point::new(7.0, 11.0);
    assert!(close(
        first.then(second).apply(point),
        second.apply(first.apply(point))
    ));
    assert!(close(Transform::IDENTITY.apply(point), point));
}

#[test]
fn bounds_measure_and_combine() {
    let bounds = Bounds::of([Point::new(3.0, -2.0), Point::new(-1.0, 4.0)]).unwrap();
    assert_eq!(bounds.min, Point::new(-1.0, -2.0));
    assert_eq!((bounds.length_mm(), bounds.width_mm()), (4.0, 6.0));
    assert_eq!(bounds.center(), Point::new(1.0, 1.0));
    assert_eq!(Bounds::of(Vec::new()), None);

    let card = Bounds::from_size((100.0, 50.0));
    assert!(card.encloses(card.expanded(-10.0)));
    assert!(!card.expanded(-10.0).encloses(card));
    assert_eq!(card.union(bounds).min, Point::new(-1.0, -2.0));
    assert!(
        card.to_polygon().signed_area_mm2() > 0.0,
        "clockwise on screen"
    );
}

#[test]
fn convex_hull_drops_the_inside_corner() {
    let l_shape = polygon(&[
        [0.0, 0.0],
        [60.0, 0.0],
        [60.0, 20.0],
        [25.0, 20.0],
        [25.0, 50.0],
        [0.0, 50.0],
    ]);
    let hull = l_shape.convex_hull();
    assert_eq!(hull.points().len(), 5);
    assert!(!hull.points().contains(&Point::new(25.0, 20.0)));
    assert!(hull.signed_area_mm2() > 0.0, "clockwise on screen");
}

#[test]
fn straightening_undoes_the_angle_of_a_photo() {
    let straight = rectangle_at(Point::new(-25.0, -15.0), (50.0, 30.0));
    let photographed = straight.transformed(Transform::turn(30.0));
    let turn = photographed.straightening_turn_deg();
    assert!((turn + 30.0).abs() < 1e-9, "{turn}");

    let bounds = photographed
        .transformed(Transform::turn(turn))
        .bounds()
        .unwrap();
    assert!((bounds.length_mm() - 50.0).abs() < 1e-9);
    assert!((bounds.width_mm() - 30.0).abs() < 1e-9);
}

#[test]
fn straightening_puts_the_long_side_along_x() {
    let tall = rectangle_at(Point::ORIGIN, (18.0, 90.0));
    assert_eq!(tall.straightening_turn_deg(), 90.0);
    let wide = rectangle_at(Point::ORIGIN, (90.0, 18.0));
    assert_eq!(wide.straightening_turn_deg(), 0.0);
}

#[test]
fn round_outlines_barely_turn() {
    let circle = Polygon(Path::circle(Point::ORIGIN, 26.0).flatten(CURVE_TOLERANCE_MM));
    let turn = circle.straightening_turn_deg();
    assert!(turn.abs() < 1.0, "{turn}");
}

#[test]
fn circle_path_starts_on_the_right() {
    let circle = Path::circle(Point::new(10.0, 5.0), 3.0);
    assert_eq!(circle.start, Point::new(13.0, 5.0));
    for point in circle.flatten(CURVE_TOLERANCE_MM) {
        assert!((point.distance_to(Point::new(10.0, 5.0)) - 3.0).abs() < 1e-9);
    }
}

#[test]
fn union_all_matches_joining_one_by_one() {
    let squares: Vec<Area> = [[0.0, 0.0], [5.0, 5.0], [30.0, 0.0], [12.0, 3.0]]
        .iter()
        .map(|&corner| {
            Area::from_polygon(&rectangle_at(Point::from(corner), (10.0, 10.0))).unwrap()
        })
        .collect();
    let all_at_once = Area::union_all(&squares).unwrap();
    let one_by_one = squares
        .iter()
        .try_fold(Area::default(), |joined, square| joined.union(square))
        .unwrap();
    assert!((all_at_once.area_mm2() - one_by_one.area_mm2()).abs() < 1e-6);
    assert_eq!(all_at_once.regions().len(), one_by_one.regions().len());
    assert!(Area::union_all(&[]).unwrap().is_empty());
}

#[test]
fn transformed_area_keeps_its_size_and_ring_directions() {
    let frame = Area::from_polygon(&rectangle_at(Point::ORIGIN, (50.0, 50.0)))
        .unwrap()
        .difference(
            &Area::from_polygon(&rectangle_at(Point::new(10.0, 10.0), (30.0, 30.0))).unwrap(),
        )
        .unwrap();
    let moved = frame.transformed(Transform {
        turn_deg: 37.0,
        shift: Point::new(-8.0, 12.0),
    });
    assert!((moved.area_mm2() - frame.area_mm2()).abs() < 1e-6);
    let [region] = moved.regions() else {
        panic!("expected one region");
    };
    assert!(region.outer.signed_area_mm2() > 0.0);
    assert!(region.holes[0].signed_area_mm2() < 0.0);
}

#[test]
fn polygon_contains_points_inside_only() {
    let l_shape = common::measure::l_shape();
    assert!(l_shape.contains(Point::new(10.0, 10.0)));
    assert!(l_shape.contains(Point::new(10.0, 40.0)));
    // The L's missing corner, and points beyond it.
    assert!(!l_shape.contains(Point::new(40.0, 40.0)));
    assert!(!l_shape.contains(Point::new(-1.0, 10.0)));
    assert_eq!(l_shape.edges().count(), 6);
}

#[test]
fn line_distance_is_zero_when_lines_cross_or_touch() {
    let line = |a: [f64; 2], b: [f64; 2]| insert_core::Line {
        from: Point::from(a),
        to: Point::from(b),
    };
    let across = line([0.0, 0.0], [10.0, 0.0]);
    assert_eq!(across.distance_to_line(line([5.0, -3.0], [5.0, 3.0])), 0.0);
    // One end resting on the other line.
    assert_eq!(across.distance_to_line(line([5.0, 0.0], [5.0, 3.0])), 0.0);
    // Parallel, 2 apart; then skew, closest end to end.
    assert!((across.distance_to_line(line([2.0, 2.0], [8.0, 2.0])) - 2.0).abs() < 1e-12);
    assert!((across.distance_to_line(line([13.0, 4.0], [20.0, 9.0])) - 5.0).abs() < 1e-12);
}

#[test]
fn bounds_gap_is_zero_when_boxes_meet() {
    let a = Bounds::from_size((10.0, 10.0));
    let shifted = |x: f64, y: f64| Bounds {
        min: Point::new(x, y),
        max: Point::new(x + 5.0, y + 5.0),
    };
    assert_eq!(a.gap_to(shifted(10.0, 2.0)), 0.0);
    assert_eq!(a.gap_to(shifted(4.0, 4.0)), 0.0);
    assert!((a.gap_to(shifted(13.0, 14.0)) - 5.0).abs() < 1e-12);
}
