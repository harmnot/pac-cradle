//! M1-A10: the browser preview's copy of a packaging (CLAUDE.md §7.4). Lines are
//! straight, every point sits on the 0.01 mm grid, every line stays within 0.06 mm of the
//! true one, copies of one hole come out alike, and sizes, placements and mounts stay
//! exact.
// The helpers below are test code too; clippy.toml only exempts `#[test]` functions.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use insert_core::{
    Align, Board, Bounds, BoxSize, BoxSpec, BoxStyle, CutoutCardSpec, Design, FingerNotches,
    InsertSpec, Line, Material, PREVIEW_GRID_MM, PREVIEW_TOLERANCE_MM, Packaging, Path, Piece,
    Point, Segment,
};

/// How far apart copies of one hole may draw: well under anything visible at the
/// editor's zoom, and twice what each may stray from the true hole.
const ALIKE_MM: f64 = 0.15;

/// How far a preview line may be from the true one: the tolerance, plus rounding to the
/// grid (half a step each way, so under one step across), plus the exact reference's own
/// chords.
const LIMIT_MM: f64 = PREVIEW_TOLERANCE_MM + PREVIEW_GRID_MM + 0.002;

/// The three sample designs, each with a box: both box styles and all three inserts.
fn designs() -> Vec<Design> {
    let mut grid = common::grid_design();
    grid.outer_box = Some(BoxSpec {
        style: BoxStyle::Shipping,
        material: Material::starting_values(Board::BFlute),
        inside_mm: None,
    });
    // The card's fixed box, tall enough for its 95 mm products.
    let mut card = common::card_design();
    if let Some(spec) = &mut card.outer_box {
        spec.inside_mm = spec.inside_mm.map(|size| BoxSize {
            height_mm: 100.0,
            ..size
        });
    }
    vec![grid, card, common::cradle_design()]
}

fn generate(design: &Design) -> Packaging {
    match design.generate() {
        Ok(packaging) => packaging,
        Err(error) => panic!("generate: {error}"),
    }
}

fn pieces(packaging: &Packaging) -> Vec<&Piece> {
    let box_pieces = packaging
        .outer_box
        .iter()
        .flat_map(|outer_box| &outer_box.pieces);
    packaging.insert.pieces.iter().chain(box_pieces).collect()
}

/// Every drawn path of a piece: outline, holes, creases, perforations.
fn paths(piece: &Piece) -> Vec<&Path> {
    std::iter::once(&piece.outline)
        .chain(&piece.holes)
        .chain(&piece.creases)
        .chain(&piece.perforations)
        .collect()
}

/// A preview path's points; panics on an arc, which the preview never has.
fn line_points(path: &Path) -> Vec<Point> {
    let ends = path.segments.iter().map(|segment| match *segment {
        Segment::Line { to } => to,
        Segment::Arc { .. } => panic!("an arc in the preview"),
    });
    std::iter::once(path.start).chain(ends).collect()
}

fn on_grid(value: f64) -> bool {
    let steps = value / PREVIEW_GRID_MM;
    (steps - steps.round()).abs() < 1e-6 && !(value == 0.0 && value.is_sign_negative())
}

fn assert_on_grid(points: &[Point], what: &str) {
    for point in points {
        assert!(
            on_grid(point.x) && on_grid(point.y),
            "{what}: {point:?} is off the grid"
        );
    }
}

/// No point repeats the one before it, nor (round a closed ring) the first.
fn assert_no_repeats((points, closed, what): (&[Point], bool, &str)) {
    for pair in points.windows(2) {
        assert_ne!(pair[0], pair[1], "{what}: a point repeats");
    }
    if closed && points.len() > 1 {
        assert_ne!(
            points.first(),
            points.last(),
            "{what}: the ring repeats its start"
        );
    }
}

#[test]
fn every_drawn_point_is_on_the_grid_without_repeats() {
    for design in designs() {
        let preview = generate(&design).rounded_for_preview();
        for piece in pieces(&preview) {
            for path in paths(piece) {
                let points = line_points(path);
                assert_on_grid(&points, &piece.name);
                assert_no_repeats((&points, path.closed, &piece.name));
            }
            for panel in &piece.panels {
                for ring in std::iter::once(&panel.outline).chain(&panel.holes) {
                    assert_on_grid(ring.points(), &piece.name);
                    assert_no_repeats((ring.points(), true, &piece.name));
                }
            }
            for hinge in &piece.hinges {
                assert_on_grid(&[hinge.axis.from, hinge.axis.to], &piece.name);
            }
        }
    }
}

/// The segments a run of points draws, joined back to the start when `closed`.
fn segments((points, closed): (&[Point], bool)) -> Vec<Line> {
    let mut lines: Vec<Line> = points
        .windows(2)
        .map(|pair| Line {
            from: pair[0],
            to: pair[1],
        })
        .collect();
    if let (true, Some(&first), Some(&last)) = (closed, points.first(), points.last()) {
        lines.push(Line {
            from: last,
            to: first,
        });
    }
    lines
}

/// The farthest any point of `from` lies from the line `to` draws.
fn farthest((from, to): (&[Point], &[Line])) -> f64 {
    from.iter()
        .map(|&point| {
            to.iter()
                .map(|line| line.distance_to(point))
                .fold(f64::INFINITY, f64::min)
        })
        .fold(0.0, f64::max)
}

/// How far apart two drawn lines get, measured both ways.
fn apart((a, b, closed): (&[Point], &[Point], bool)) -> f64 {
    farthest((a, &segments((b, closed)))).max(farthest((b, &segments((a, closed)))))
}

#[test]
fn every_line_stays_close_to_the_true_one() {
    for design in designs() {
        let exact = generate(&design);
        let preview = exact.rounded_for_preview();
        for (exact_piece, preview_piece) in pieces(&exact).into_iter().zip(pieces(&preview)) {
            for (exact_path, preview_path) in
                paths(exact_piece).into_iter().zip(paths(preview_piece))
            {
                let want = exact_path.flatten(0.001);
                let got = line_points(preview_path);
                let gap = apart((&want, &got, exact_path.closed));
                assert!(gap <= LIMIT_MM, "{}: {gap} mm off", exact_piece.name);
                assert_eq!(exact_path.closed, preview_path.closed);
            }
        }
    }
}

#[test]
fn copies_of_one_hole_come_out_alike() {
    // Six notched holes for one jar: each copy sits elsewhere on the grid, but they
    // must draw the same (the first, 0.5 mm grid gave each its own stair-steps).
    let mut jar = common::product("jar", common::circle(60.0));
    jar.quantity = 6;
    let design = Design {
        material: Material::starting_values(Board::EFlute),
        products: vec![jar],
        insert: InsertSpec::CutoutCard(CutoutCardSpec {
            margin_mm: 10.0,
            finger_notches: Some(FingerNotches::default()),
            align: Align::Center,
            placements: Vec::new(),
        }),
        outer_box: None,
    };
    let preview = generate(&design).rounded_for_preview();
    let holes: Vec<Vec<Point>> = preview.insert.pieces[0]
        .holes
        .iter()
        .map(|hole| {
            // Each hole moved so its box starts at (0, 0).
            let points = line_points(hole);
            let corner = Bounds::of(points.iter().copied()).unwrap().min;
            points.into_iter().map(|point| point - corner).collect()
        })
        .collect();
    assert_eq!(holes.len(), 6);
    for hole in &holes[1..] {
        let gap = apart((&holes[0], hole, true));
        assert!(gap <= ALIKE_MM, "copies {gap} mm apart");
    }
}

#[test]
fn sizes_placements_and_mounts_stay_exact() {
    for design in designs() {
        let exact = generate(&design);
        let preview = exact.rounded_for_preview();
        let (insert, rounded) = (&exact.insert, &preview.insert);
        assert_eq!(
            [insert.length_mm, insert.width_mm, insert.height_mm],
            [rounded.length_mm, rounded.width_mm, rounded.height_mm]
        );
        assert_eq!(insert.placements, rounded.placements);
        assert_eq!(insert.mounts, rounded.mounts);
        let outer_box = exact.outer_box.as_ref().unwrap();
        let rounded_box = preview.outer_box.as_ref().unwrap();
        assert_eq!(outer_box.inside_mm, rounded_box.inside_mm);
        assert_eq!(outer_box.outside_mm, rounded_box.outside_mm);
        assert_eq!(outer_box.mounts, rounded_box.mounts);

        // Same pieces, with the same lines and folds; only their points move.
        for (exact_piece, preview_piece) in pieces(&exact).into_iter().zip(pieces(&preview)) {
            assert_eq!(exact_piece.name, preview_piece.name);
            assert_eq!(exact_piece.quantity, preview_piece.quantity);
            assert_eq!(exact_piece.holes.len(), preview_piece.holes.len());
            assert_eq!(exact_piece.creases.len(), preview_piece.creases.len());
            assert_eq!(exact_piece.panels.len(), preview_piece.panels.len());
            assert_eq!(exact_piece.hinges.len(), preview_piece.hinges.len());
            for (want, got) in exact_piece.hinges.iter().zip(&preview_piece.hinges) {
                assert_eq!(
                    (want.parent, want.child, want.direction, want.stage),
                    (got.parent, got.child, got.direction, got.stage)
                );
                assert_eq!(
                    [want.fold_deg, want.offset_mm],
                    [got.fold_deg, got.offset_mm]
                );
            }
        }
    }
}
