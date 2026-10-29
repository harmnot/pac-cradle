//! M1-B3: the DXF production file (CLAUDE.md §6.6). It's DXF R12 in mm; CUT is red and
//! CREASE blue, with PERFORATION only when a piece has some; every path is one polyline
//! on its layer with the sheet's points, y flipped up; arcs become bulges on the true
//! circle, turned the other way when mirrored. Two whole files are golden
//! (`tests/golden/export_*.dxf`).
//!
//! Needs the `export` feature: `cargo test -p insert_core --features export`.
#![cfg(feature = "export")]
// The helpers below are test code too; clippy.toml only exempts `#[test]` functions.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use insert_core::export::{LineKind, Sheet, Spacing};
use insert_core::{
    Board, Bounds, BoxSpec, BoxStyle, CURVE_TOLERANCE_MM, Design, Material, Packaging, Point,
};

/// Coordinates are written to 0.0001 mm.
const WRITTEN_MM: f64 = 1e-4;

/// The engine's sample: two bottles on a chipboard cradle in an E-flute mailer.
fn cradle_mailer() -> Packaging {
    let design: Design = serde_json::from_str(include_str!("fixtures/cradle_mailer.json")).unwrap();
    design.generate().unwrap()
}

/// Six jars in an E-flute grid, in a B-flute shipping box.
fn grid_shipping() -> Packaging {
    let design = Design {
        outer_box: Some(BoxSpec {
            style: BoxStyle::Shipping,
            material: Material::starting_values(Board::BFlute),
            inside_mm: None,
        }),
        ..common::grid_design()
    };
    design.generate().unwrap()
}

/// The file's groups: (code, value) pairs, a line each.
fn pairs(dxf: &str) -> Vec<(u16, &str)> {
    let lines: Vec<&str> = dxf.lines().collect();
    assert_eq!(lines.len() % 2, 0, "a group code without a value");
    lines
        .chunks(2)
        .map(|pair| {
            let code = pair[0]
                .trim()
                .parse()
                .unwrap_or_else(|_| panic!("not a group code: {:?}", pair[0]));
            (code, pair[1])
        })
        .collect()
}

/// A header variable's value.
fn header<'a>((pairs, name): (&[(u16, &'a str)], &str)) -> &'a str {
    let at = pairs.iter().position(|&pair| pair == (9, name)).unwrap();
    pairs[at + 1].1
}

/// A polyline read back from the file.
#[derive(Debug)]
struct Polyline {
    layer: String,
    closed: bool,
    /// Each vertex and the bulge of the stretch it starts.
    vertices: Vec<(Point, f64)>,
}

fn polylines(pairs: &[(u16, &str)]) -> Vec<Polyline> {
    let mut polylines = Vec::new();
    let mut entities = pairs
        .iter()
        .skip_while(|&&pair| pair != (2, "ENTITIES"))
        .peekable();
    while let Some(&(code, value)) = entities.next() {
        if (code, value) != (0, "POLYLINE") {
            continue;
        }
        let mut polyline = Polyline {
            layer: String::new(),
            closed: false,
            vertices: Vec::new(),
        };
        let mut vertex: Option<(f64, f64, f64)> = None;
        for &(code, value) in entities.by_ref() {
            match code {
                0 => {
                    if let Some((x, y, bulge)) = vertex.take() {
                        polyline.vertices.push((Point::new(x, y), bulge));
                    }
                    match value {
                        "VERTEX" => vertex = Some((0.0, 0.0, 0.0)),
                        "SEQEND" => break,
                        other => panic!("unexpected {other} in a polyline"),
                    }
                }
                8 if polyline.layer.is_empty() => polyline.layer = value.to_owned(),
                70 if vertex.is_none() => polyline.closed = value == "1",
                10 => vertex.iter_mut().for_each(|v| v.0 = value.parse().unwrap()),
                20 => vertex.iter_mut().for_each(|v| v.1 = value.parse().unwrap()),
                42 => vertex.iter_mut().for_each(|v| v.2 = value.parse().unwrap()),
                _ => {}
            }
        }
        polylines.push(polyline);
    }
    polylines
}

#[test]
fn file_is_dxf_r12_in_millimetres() {
    for packaging in [cradle_mailer(), common::arc_packaging()] {
        let sheet = Sheet::of((&packaging, Spacing::CUT_FILE));
        let dxf = packaging.to_dxf();
        let pairs = pairs(&dxf);
        assert_eq!(header((&pairs, "$ACADVER")), "AC1009");
        assert_eq!(header((&pairs, "$INSUNITS")), "4");
        assert_eq!(header((&pairs, "$MEASUREMENT")), "1");

        let sections: Vec<&str> = pairs
            .windows(2)
            .filter(|pair| pair[0] == (0, "SECTION"))
            .map(|pair| pair[1].1)
            .collect();
        assert_eq!(sections, ["HEADER", "TABLES", "ENTITIES"]);
        assert_eq!(pairs.last(), Some(&(0, "EOF")));

        // The extent is the sheet, from the origin.
        let at = pairs
            .iter()
            .position(|&pair| pair == (9, "$EXTMAX"))
            .unwrap();
        let extent: Vec<f64> = pairs[at + 1..at + 3]
            .iter()
            .map(|(_, value)| value.parse().unwrap())
            .collect();
        assert!((extent[0] - sheet.length_mm).abs() < WRITTEN_MM);
        assert!((extent[1] - sheet.width_mm).abs() < WRITTEN_MM);
        assert!(!dxf.contains("TEXT"), "a cut file has no text");
    }
}

/// The layers and their colours, in the table's order.
fn layers(pairs: &[(u16, &str)]) -> Vec<(String, String)> {
    pairs
        .windows(8)
        .filter(|window| window[0] == (0, "LAYER") && window[1].0 == 2)
        .map(|window| {
            let color = window.iter().find(|pair| pair.0 == 62).unwrap().1;
            (window[1].1.to_owned(), color.trim().to_owned())
        })
        .collect()
}

#[test]
fn cut_is_red_crease_blue_and_perforation_green_only_when_needed() {
    let want = |names: &[(&str, &str)]| -> Vec<(String, String)> {
        names
            .iter()
            .map(|(name, color)| ((*name).to_owned(), (*color).to_owned()))
            .collect()
    };
    let plain = cradle_mailer().to_dxf();
    assert_eq!(
        layers(&pairs(&plain)),
        want(&[("CUT", "1"), ("CREASE", "5")])
    );

    let dxf = common::arc_packaging().to_dxf();
    assert_eq!(
        layers(&pairs(&dxf)),
        want(&[("CUT", "1"), ("CREASE", "5"), ("PERFORATION", "3")])
    );
}

#[test]
fn every_path_is_one_polyline_with_the_sheets_points() {
    for packaging in [cradle_mailer(), grid_shipping()] {
        let sheet = Sheet::of((&packaging, Spacing::CUT_FILE));
        let dxf = packaging.to_dxf();
        let read = polylines(&pairs(&dxf));
        let flip = |point: Point| Point::new(point.x, sheet.width_mm - point.y);

        let mut read = read.iter();
        for kind in [LineKind::Cut, LineKind::Crease] {
            for path in sheet.pieces.iter().flat_map(|piece| piece.lines(kind)) {
                let polyline = read.next().unwrap();
                assert_eq!(polyline.layer, kind.layer_name());
                assert_eq!(polyline.closed, path.closed);
                let want: Vec<Point> = path
                    .flatten(CURVE_TOLERANCE_MM)
                    .into_iter()
                    .map(flip)
                    .collect();
                assert_eq!(polyline.vertices.len(), want.len());
                for ((got, bulge), want) in polyline.vertices.iter().zip(&want) {
                    assert!(
                        got.distance_to(*want) < 2.0 * WRITTEN_MM,
                        "{got:?} vs {want:?}"
                    );
                    assert_eq!(*bulge, 0.0, "the samples have no arcs");
                }
            }
        }
        assert!(read.next().is_none(), "more polylines than paths");
    }
}

/// The point halfway along a stretch from `from` to `to` with `bulge`: the chord's middle,
/// pushed out by the sagitta (half the bulge times the chord), to the right of the chord
/// for a positive (anticlockwise, y up) bulge.
fn stretch_middle((from, to, bulge): (Point, Point, f64)) -> Point {
    let (dx, dy) = (to.x - from.x, to.y - from.y);
    Point::new(
        (from.x + to.x) / 2.0 + bulge / 2.0 * dy,
        (from.y + to.y) / 2.0 - bulge / 2.0 * dx,
    )
}

#[test]
fn arcs_become_bulges_on_the_true_circle() {
    let packaging = common::arc_packaging();
    let sheet = Sheet::of((&packaging, Spacing::CUT_FILE));
    let read = polylines(&pairs(&packaging.to_dxf()));
    let cuts: Vec<&Polyline> = read.iter().filter(|p| p.layer == "CUT").collect();
    let creases: Vec<&Polyline> = read.iter().filter(|p| p.layer == "CREASE").collect();
    // The disc in quarter turns, closed back to its start; the crease in one quarter.
    assert_eq!(
        cuts.iter().map(|p| p.vertices.len()).collect::<Vec<_>>(),
        [4, 4]
    );
    assert!(cuts.iter().all(|p| p.closed));
    assert_eq!(
        creases.iter().map(|p| p.vertices.len()).collect::<Vec<_>>(),
        [2, 2]
    );

    let own = Bounds::of(
        packaging.insert.pieces[0]
            .outline
            .flatten(CURVE_TOLERANCE_MM),
    )
    .unwrap();
    for (index, piece) in sheet.pieces.iter().enumerate() {
        // The disc's centre, (30, 30) in its own drawing, where the sheet put it, y up.
        let x = match piece.mirrored {
            true => own.max.x - 30.0,
            false => 30.0 - own.min.x,
        };
        let center = Point::new(
            piece.bounds.min.x + x,
            sheet.width_mm - (piece.bounds.min.y + 30.0 - own.min.y),
        );
        for (polyline, radius) in [(cuts[index], 30.0), (creases[index], 20.0)] {
            let count = polyline.vertices.len();
            let stretches = if polyline.closed { count } else { count - 1 };
            for at in 0..stretches {
                let (from, bulge) = polyline.vertices[at];
                let to = polyline.vertices[(at + 1) % count].0;
                assert!(bulge != 0.0, "a straight stretch on a circle");
                for point in [from, to, stretch_middle((from, to, bulge))] {
                    let off = point.distance_to(center) - radius;
                    assert!(off.abs() < 1e-3, "{} off by {off} mm", piece.name);
                }
            }
        }
    }
    // The crease turns clockwise on screen on the insert (a negative bulge with y up),
    // and the other way on the mirrored copy.
    let quarter = (22.5_f64).to_radians().tan();
    assert!((creases[0].vertices[0].1 + quarter).abs() < WRITTEN_MM);
    assert!((creases[1].vertices[0].1 - quarter).abs() < WRITTEN_MM);
}

#[test]
fn whole_files_match_their_golden_copies() {
    let failures: Vec<String> = [
        ("export_cradle_mailer.dxf", cradle_mailer()),
        ("export_grid_shipping.dxf", grid_shipping()),
    ]
    .into_iter()
    .filter_map(|(file, packaging)| common::golden::check((file, &packaging.to_dxf())).err())
    .collect();
    assert!(failures.is_empty(), "\n{}", failures.join("\n\n"));
}
