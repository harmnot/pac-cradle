//! M1-B1: the SVG production file (CLAUDE.md §6.6). Every piece is laid out once with
//! nothing overlapping; insert pieces keep their shape and box pieces come out mirrored,
//! print side up; the file has CUT and CREASE layers in mm, and no drawn text. Two whole
//! files are golden (`tests/golden/export_*.svg`).
//!
//! Needs the `export` feature: `cargo test -p insert_core --features export` (the
//! workspace turns it on through the API crate).
#![cfg(feature = "export")]
// The helpers below are test code too; clippy.toml only exempts `#[test]` functions.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use insert_core::export::{LineKind, Sheet, Spacing};
use insert_core::{
    Board, BoxSize, BoxSpec, BoxStyle, Design, Material, Packaging, Part, Path, Piece, Point,
    Segment,
};

/// Paths compare to this after flattening: the same arithmetic, give or take rounding.
const SAME_MM: f64 = 1e-9;

/// The engine's sample: two bottles on a chipboard cradle in an E-flute mailer.
fn cradle_mailer() -> Design {
    serde_json::from_str(include_str!("fixtures/cradle_mailer.json")).unwrap()
}

/// Six jars in an E-flute grid, in a B-flute shipping box.
fn grid_shipping() -> Design {
    Design {
        outer_box: Some(BoxSpec {
            style: BoxStyle::Shipping,
            material: Material::starting_values(Board::BFlute),
            inside_mm: None,
        }),
        ..common::grid_design()
    }
}

/// The lip gloss and compact card, in its fixed shipping box made tall enough.
fn card_shipping() -> Design {
    let mut design = common::card_design();
    if let Some(spec) = &mut design.outer_box {
        spec.inside_mm = spec.inside_mm.map(|size| BoxSize {
            height_mm: 100.0,
            ..size
        });
    }
    design
}

fn generate(design: &Design) -> Packaging {
    match design.generate() {
        Ok(packaging) => packaging,
        Err(error) => panic!("generate: {error}"),
    }
}

/// The piece a sheet piece came from.
fn source((packaging, part, index): (&Packaging, Part, usize)) -> &Piece {
    match part {
        Part::Insert => &packaging.insert.pieces[index],
        Part::OuterBox => &packaging.outer_box.as_ref().unwrap().pieces[index],
    }
}

/// A piece's lines of one kind, as the engine drew them.
fn source_lines((piece, kind): (&Piece, LineKind)) -> Vec<&Path> {
    match kind {
        LineKind::Cut => std::iter::once(&piece.outline)
            .chain(&piece.holes)
            .collect(),
        LineKind::Crease => piece.creases.iter().collect(),
        LineKind::Perforation => piece.perforations.iter().collect(),
    }
}

fn assert_same_points((want, got, what): (&[Point], &[Point], &str)) {
    assert_eq!(want.len(), got.len(), "{what}: point count");
    for (a, b) in want.iter().zip(got) {
        assert!(a.distance_to(*b) <= SAME_MM, "{what}: {a:?} became {b:?}");
    }
}

/// Every line of every sheet piece is its source line moved to the piece's spot, and
/// mirrored left to right for box pieces.
fn assert_lines_follow_their_pieces(packaging: &Packaging) {
    let sheet = Sheet::of((packaging, Spacing::CUT_FILE));
    for placed in &sheet.pieces {
        let piece = source((packaging, placed.part, placed.index));
        let own = insert_core::Bounds::of(piece.outline.flatten(0.002)).unwrap();
        let moved = |point: Point| {
            let x = match placed.mirrored {
                true => own.max.x - point.x,
                false => point.x - own.min.x,
            };
            Point::new(
                placed.bounds.min.x + x,
                placed.bounds.min.y + point.y - own.min.y,
            )
        };
        for kind in LineKind::ALL {
            let want = source_lines((piece, kind));
            let got = placed.lines(kind);
            assert_eq!(want.len(), got.len(), "{}: {kind:?} lines", placed.name);
            for (want, got) in want.iter().zip(got) {
                let points: Vec<Point> = want.flatten(0.002).into_iter().map(moved).collect();
                assert_same_points((&points, &got.flatten(0.002), &placed.name));
                assert_eq!(want.closed, got.closed, "{}", placed.name);
            }
        }
    }
}

#[test]
fn every_piece_is_laid_out_once_without_overlap() {
    for design in [cradle_mailer(), grid_shipping(), card_shipping()] {
        let packaging = generate(&design);
        let sheet = Sheet::of((&packaging, Spacing::CUT_FILE));
        let box_pieces = packaging.outer_box.as_ref().unwrap().pieces.len();
        assert_eq!(
            sheet.pieces.len(),
            packaging.insert.pieces.len() + box_pieces
        );

        // Insert pieces first, then the box, each in the order generated; only the box
        // is mirrored.
        let order: Vec<(Part, usize, bool)> = sheet
            .pieces
            .iter()
            .map(|piece| (piece.part, piece.index, piece.mirrored))
            .collect();
        let insert = (0..packaging.insert.pieces.len()).map(|index| (Part::Insert, index, false));
        let outer_box = (0..box_pieces).map(|index| (Part::OuterBox, index, true));
        assert_eq!(order, insert.chain(outer_box).collect::<Vec<_>>());

        for (index, piece) in sheet.pieces.iter().enumerate() {
            let bounds = piece.bounds;
            let Spacing { margin_mm, gap_mm } = Spacing::CUT_FILE;
            assert!(bounds.min.x >= margin_mm && bounds.min.y >= margin_mm);
            assert!(bounds.max.x <= sheet.length_mm - margin_mm + SAME_MM);
            assert!(bounds.max.y <= sheet.width_mm - margin_mm + SAME_MM);
            for other in &sheet.pieces[index + 1..] {
                let gap = bounds.gap_to(other.bounds);
                assert!(gap >= gap_mm - SAME_MM, "{} and {}", piece.name, other.name);
            }
        }
    }
}

#[test]
fn inserts_keep_their_shape_and_boxes_are_mirrored_print_side_up() {
    for design in [cradle_mailer(), grid_shipping(), card_shipping()] {
        assert_lines_follow_their_pieces(&generate(&design));
    }
}

#[test]
fn arcs_stay_arcs_and_turn_the_other_way_when_mirrored() {
    let packaging = common::arc_packaging();
    assert_lines_follow_their_pieces(&packaging);

    let sheet = Sheet::of((&packaging, Spacing::CUT_FILE));
    let sweeps: Vec<f64> = sheet
        .pieces
        .iter()
        .map(|piece| match piece.creases[0].segments[0] {
            Segment::Arc { sweep_deg, .. } => sweep_deg,
            Segment::Line { .. } => panic!("the arc became a line"),
        })
        .collect();
    assert_eq!(sweeps, [90.0, -90.0]);

    // The full circle is written as two half-turn arcs; the mirrored crease's arc has
    // SVG's sweep flag 0 (anticlockwise on screen).
    let svg = sheet.to_svg();
    let circle = svg
        .lines()
        .find(|line| line.starts_with("<path") && line.contains('Z'))
        .unwrap();
    assert_eq!(circle.matches(" A").count(), 2, "{circle}");
    let creases: Vec<&str> = layer_lines((&svg, LineKind::Crease))
        .into_iter()
        .filter(|line| line.starts_with("<path"))
        .collect();
    assert_eq!(creases.len(), 2);
    assert!(creases[0].contains(" A20 20 0 0 1 "), "{}", creases[0]);
    assert!(creases[1].contains(" A20 20 0 0 0 "), "{}", creases[1]);
}

/// The lines of one layer of an SVG file, up to the next layer.
fn layer_lines((svg, kind): (&str, LineKind)) -> Vec<&str> {
    let start = format!("<g id=\"{}\"", kind.layer_name());
    let is_layer = |line: &&str| line.contains("inkscape:groupmode=\"layer\"");
    svg.lines()
        .skip_while(|line| !line.starts_with(&start))
        .skip(1)
        .take_while(|line| !is_layer(line))
        .collect()
}

#[test]
fn svg_has_cut_and_crease_layers_in_mm_and_no_text() {
    let packaging = generate(&cradle_mailer());
    let sheet = Sheet::of((&packaging, Spacing::CUT_FILE));
    let svg = packaging.to_svg();

    assert!(svg.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<svg "));
    // Sized in mm, one user unit per mm.
    let (length, width) = (sheet.length_mm, sheet.width_mm);
    let root = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" \
         xmlns:inkscape=\"http://www.inkscape.org/namespaces/inkscape\" version=\"1.1\" \
         width=\"{length}mm\" height=\"{width}mm\" viewBox=\"0 0 {length} {width}\">"
    );
    let line = svg.lines().nth(1).unwrap();
    assert!(common::golden::same_line((&root, line)), "{line}");
    assert!(!svg.contains("<text"));

    for kind in [LineKind::Cut, LineKind::Crease] {
        let header = format!(
            "<g id=\"{name}\" inkscape:groupmode=\"layer\" inkscape:label=\"{name}\" \
             fill=\"none\" stroke=\"{}\" stroke-width=\"0.025\">",
            kind.color(),
            name = kind.layer_name()
        );
        assert!(svg.contains(&header), "{header}");
        let paths = layer_lines((&svg, kind))
            .into_iter()
            .filter(|line| line.starts_with("<path"))
            .count();
        let lines: usize = sheet
            .pieces
            .iter()
            .map(|piece| piece.lines(kind).len())
            .sum();
        assert_eq!(paths, lines, "{kind:?}");
    }
    // No piece has perforations, so there's no layer for them.
    assert!(!svg.contains("<g id=\"PERFORATION\""));
    assert!(
        common::arc_packaging()
            .to_svg()
            .contains("<g id=\"PERFORATION\"")
    );

    // Each piece's group is titled with how many to cut, and box pieces say they're
    // mirrored.
    assert!(svg.contains("<title>Insert: cradle x 1</title>"));
    assert!(svg.contains("<title>Box: mailer x 1 (mirrored: print side up)</title>"));
}

#[test]
fn whole_files_match_their_golden_copies() {
    let failures: Vec<String> = [
        ("export_cradle_mailer.svg", cradle_mailer()),
        ("export_grid_shipping.svg", grid_shipping()),
    ]
    .into_iter()
    .filter_map(|(file, design)| common::golden::check((file, &generate(&design).to_svg())).err())
    .collect();
    assert!(failures.is_empty(), "\n{}", failures.join("\n\n"));
}
