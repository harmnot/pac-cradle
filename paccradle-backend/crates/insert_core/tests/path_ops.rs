//! Arc flattening, path ⇄ polygon conversion, simplification and distances (M1-A2).

mod common;

use common::measure::{circle_path, distance_to_outline, edges, rectangle_at};
use insert_core::{CURVE_TOLERANCE_MM, Line, Path, Point, Polygon, Segment};

fn polygon(points: &[[f64; 2]]) -> Polygon {
    Polygon(points.iter().copied().map(Point::from).collect())
}

/// A 40 × 20 rectangle with 5 mm round corners, drawn with lines and quarter arcs.
fn rounded_rectangle() -> Path {
    let line = |x, y| Segment::Line {
        to: Point::new(x, y),
    };
    let quarter = |x, y| Segment::Arc {
        center: Point::new(x, y),
        sweep_deg: 90.0,
    };
    Path {
        start: Point::new(5.0, 0.0),
        segments: vec![
            line(35.0, 0.0),
            quarter(35.0, 5.0),
            line(40.0, 15.0),
            quarter(35.0, 15.0),
            line(5.0, 20.0),
            quarter(5.0, 15.0),
            line(0.0, 5.0),
            quarter(5.0, 5.0),
        ],
        closed: true,
    }
}

#[test]
fn full_circle_flattens_within_tolerance() {
    let radius = 26.0;
    let outline = Polygon(circle_path(radius).flatten(CURVE_TOLERANCE_MM));
    let origin = Point::new(0.0, 0.0);

    for corner in outline.points() {
        assert!((corner.distance_to(origin) - radius).abs() < 1e-9);
    }
    for edge in edges(&outline) {
        let middle = Point::new(
            (edge.from.x + edge.to.x) / 2.0,
            (edge.from.y + edge.to.y) / 2.0,
        );
        let gap = radius - middle.distance_to(origin);
        assert!(
            (0.0..=CURVE_TOLERANCE_MM).contains(&gap),
            "chord {gap} mm inside"
        );
    }
    // The start isn't repeated at the end, and the chords aren't needlessly dense.
    let points = outline.points();
    assert!(points[0].distance_to(points[points.len() - 1]) > 0.1);
    assert!(points.len() < 400, "{} points", points.len());
}

#[test]
fn positive_sweep_turns_clockwise_on_screen() {
    let quarter = |sweep_deg| Path {
        start: Point::new(10.0, 0.0),
        segments: vec![Segment::Arc {
            center: Point::new(0.0, 0.0),
            sweep_deg,
        }],
        closed: false,
    };
    // y points down, so +x towards +y is clockwise on screen.
    let down = quarter(90.0).flatten(CURVE_TOLERANCE_MM);
    assert!(down[down.len() - 1].distance_to(Point::new(0.0, 10.0)) < 1e-9);
    let up = quarter(-90.0).flatten(CURVE_TOLERANCE_MM);
    assert!(up[up.len() - 1].distance_to(Point::new(0.0, -10.0)) < 1e-9);
}

#[test]
fn rounded_rectangle_keeps_its_area_and_drops_the_closing_point() {
    let path = rounded_rectangle();
    let points = path.flatten(CURVE_TOLERANCE_MM);
    let outline = Polygon(points.clone());

    let exact = 40.0 * 20.0 - (4.0 - std::f64::consts::PI) * 5.0 * 5.0;
    assert!((outline.signed_area_mm2().abs() - exact).abs() < 0.05);
    // The last arc ends back at the start; that point isn't listed twice.
    assert!(points[points.len() - 1].distance_to(path.start) > 0.1);
}

#[test]
fn open_path_keeps_both_ends() {
    let path = Path {
        start: Point::new(0.0, 0.0),
        segments: vec![
            Segment::Line {
                to: Point::new(10.0, 0.0),
            },
            Segment::Line {
                to: Point::new(0.0, 0.0),
            },
        ],
        closed: false,
    };
    assert_eq!(path.flatten(CURVE_TOLERANCE_MM).len(), 3);
}

#[test]
fn polygon_becomes_a_closed_path_and_back() {
    let rectangle = rectangle_at(Point::new(2.0, 3.0), (60.0, 40.0));
    let path = rectangle.to_path().unwrap();
    assert!(path.closed);
    assert_eq!(
        path.segments.len(),
        3,
        "the closing edge is implied by `closed`"
    );
    assert_eq!(Polygon(path.flatten(CURVE_TOLERANCE_MM)), rectangle);

    assert_eq!(Polygon(Vec::new()).to_path(), None);
}

#[test]
fn simplify_drops_duplicate_and_in_line_points() {
    let noisy = polygon(&[
        [0.0, 0.0],
        [5.0, 0.0],
        [5.0, 0.0],
        [10.0, 0.0],
        [10.0, 4.0],
        [10.0, 10.0],
        [0.0, 10.0],
        [0.0, 3.0],
    ]);
    let square = polygon(&[[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]]);
    assert_eq!(noisy.simplify(0.0), square);
}

#[test]
fn simplify_keeps_the_outline_within_tolerance() {
    let dense = Polygon(circle_path(26.0).flatten(CURVE_TOLERANCE_MM));
    let tolerance = 0.2;
    let simple = dense.simplify(tolerance);

    assert!(simple.points().len() * 4 < dense.points().len());
    for point in dense.points() {
        assert!(distance_to_outline(*point, &simple) <= tolerance + 1e-9);
    }
    for point in simple.points() {
        assert!(
            dense.points().contains(point),
            "only original points are kept"
        );
    }
}

#[test]
fn simplify_keeps_a_spike_wider_than_the_tolerance() {
    let with_spike = polygon(&[
        [0.0, 0.0],
        [20.0, 0.0],
        [20.0, 10.0],
        [10.5, 10.0],
        [10.0, 30.0],
        [9.5, 10.0],
        [0.0, 10.0],
    ]);
    assert_eq!(with_spike.simplify(0.2), with_spike);
}

#[test]
fn simplify_leaves_triangles_alone() {
    let triangle = polygon(&[[0.0, 0.0], [10.0, 0.0], [5.0, 0.01]]);
    assert_eq!(triangle.simplify(1.0), triangle);
}

#[test]
fn segment_distance_is_to_the_nearest_point_of_the_segment() {
    let edge = Line {
        from: Point::new(0.0, 0.0),
        to: Point::new(10.0, 0.0),
    };
    assert_eq!(edge.distance_to(Point::new(5.0, 3.0)), 3.0);
    // Past the end, the distance is to the end point, not to the endless line.
    assert_eq!(edge.distance_to(Point::new(-4.0, 3.0)), 5.0);

    let dot = Line {
        from: Point::new(1.0, 1.0),
        to: Point::new(1.0, 1.0),
    };
    assert_eq!(dot.distance_to(Point::new(4.0, 5.0)), 5.0);
}
