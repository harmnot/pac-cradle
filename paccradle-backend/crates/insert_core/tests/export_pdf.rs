//! M1-B2: the PDF production file (CLAUDE.md §6.6). The file's structure holds together
//! (every object where the cross-reference table says), the page is the sheet at true
//! size, each layer holds only its own lines with no text on CUT or CREASE, labels and
//! dimensions are there, and arcs become curves on the true circle. Two whole files are
//! golden (`tests/golden/export_cradle_mailer.pdf`).
//!
//! Needs the `export` feature: `cargo test -p insert_core --features export`.
#![cfg(feature = "export")]
// The helpers below are test code too; clippy.toml only exempts `#[test]` functions.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::pdf::numbers;

use insert_core::export::{LineKind, Sheet, Spacing};
use insert_core::{Bounds, CURVE_TOLERANCE_MM, Design, Packaging, Point};

const PT_PER_MM: f64 = 72.0 / 25.4;

/// The engine's sample: two bottles on a chipboard cradle in an E-flute mailer.
fn cradle_mailer() -> Packaging {
    let design: Design = serde_json::from_str(include_str!("fixtures/cradle_mailer.json")).unwrap();
    design.generate().unwrap()
}

fn pdf(packaging: &Packaging) -> String {
    String::from_utf8(packaging.to_pdf()).expect("the PDF is plain ASCII")
}

/// The page's drawing commands.
fn content(pdf: &str) -> &str {
    let start = pdf.find("stream\n").unwrap() + "stream\n".len();
    let end = pdf.find("endstream").unwrap();
    &pdf[start..end]
}

/// The layer names, in the catalog's order.
fn layer_names(pdf: &str) -> Vec<&str> {
    pdf.match_indices("/Type /OCG /Name (")
        .map(|(at, found)| {
            let rest = &pdf[at + found.len()..];
            &rest[..rest.find(')').unwrap()]
        })
        .collect()
}

/// Each layer's commands, by name.
fn layers(pdf: &str) -> Vec<(&str, &str)> {
    let content = content(pdf);
    layer_names(pdf)
        .into_iter()
        .enumerate()
        .map(|(index, name)| {
            let start = content.find(&format!("/OC /L{index} BDC\n")).unwrap();
            let end = start + content[start..].find("EMC\n").unwrap();
            (name, &content[start..end])
        })
        .collect()
}

fn layer<'a>((pdf, name): (&'a str, &str)) -> &'a str {
    layers(pdf)
        .into_iter()
        .find(|(layer, _)| *layer == name)
        .unwrap_or_else(|| panic!("no {name} layer"))
        .1
}

#[test]
fn every_object_is_where_the_cross_reference_table_says() {
    for packaging in [cradle_mailer(), common::arc_packaging()] {
        let pdf = pdf(&packaging);
        common::pdf::check_structure(&pdf);
        // The content stream is as long as it says.
        let length: usize = numbers(&pdf[pdf.find("<< /Length ").unwrap()..][..30])[0] as usize;
        assert_eq!(content(&pdf).len(), length);
    }
}

#[test]
fn the_page_is_the_sheet_at_true_size() {
    let packaging = cradle_mailer();
    let sheet = Sheet::of((&packaging, Spacing::DRAWING));
    let pdf = pdf(&packaging);
    let media_box = &pdf[pdf.find("/MediaBox [").unwrap()..];
    let size = numbers(&media_box["/MediaBox [".len()..media_box.find(']').unwrap()]);
    let want = [
        0.0,
        0.0,
        sheet.length_mm * PT_PER_MM,
        sheet.width_mm * PT_PER_MM,
    ];
    for (got, want) in size.iter().zip(want) {
        assert!((got - want).abs() < 1e-3, "{size:?}");
    }
    // Drawn in mm from the top-left corner: scaled by points per mm, y flipped.
    let scale = format!("{} 0 0 -{}", round(PT_PER_MM), round(PT_PER_MM));
    assert!(content(&pdf).starts_with(&format!("q\n{scale} 0 ")));
}

/// A number as the file writes it.
fn round(value: f64) -> String {
    let text = format!("{value:.4}");
    text.trim_end_matches('0').trim_end_matches('.').to_owned()
}

#[test]
fn each_layer_holds_its_own_lines_and_text_stays_off_cut_and_crease() {
    let packaging = cradle_mailer();
    let sheet = Sheet::of((&packaging, Spacing::DRAWING));
    let pdf = pdf(&packaging);
    assert_eq!(layer_names(&pdf), ["CUT", "CREASE", "DIMENSIONS", "LABELS"]);

    for kind in [LineKind::Cut, LineKind::Crease] {
        let commands = layer((&pdf, kind.layer_name()));
        let paths: usize = sheet
            .pieces
            .iter()
            .map(|piece| piece.lines(kind).len())
            .sum();
        let strokes = commands.lines().filter(|line| *line == "S").count();
        assert_eq!(strokes, paths, "{kind:?}");
        assert!(!commands.contains("Tj"), "text on {kind:?}");
    }
    assert!(layer((&pdf, "CUT")).contains("1 0 0 RG 0.1 w [] 0 d"));
    assert!(layer((&pdf, "CREASE")).contains("0 0 1 RG 0.1 w [3 1.5] 0 d"));

    // Each piece's length and width, to a tenth of a mm: the cradle is 142 × 228.8 mm.
    let dimensions = layer((&pdf, "DIMENSIONS"));
    assert_eq!(dimensions.matches(" Tj ET").count(), 2 * sheet.pieces.len());
    for piece in &sheet.pieces {
        for mm in [piece.bounds.length_mm(), piece.bounds.width_mm()] {
            assert!(dimensions.contains(&format!("({mm:.1}) Tj")), "{mm}");
        }
    }
    assert!(dimensions.contains("(142.0) Tj") && dimensions.contains("(228.8) Tj"));

    // Each piece's title, and the two legend lines.
    let labels = layer((&pdf, "LABELS"));
    assert_eq!(labels.matches(" Tj ET").count(), sheet.pieces.len() + 2);
    for piece in &sheet.pieces {
        // Brackets in a PDF string are escaped.
        let title = piece.title().replace('(', "\\(").replace(')', "\\)");
        assert!(labels.contains(&format!("({title}) Tj")), "{title}");
    }
    assert!(labels.contains("print at 100 %"));

    // Perforations get a layer only when a piece has some.
    let perforated = pdf_names(&common::arc_packaging());
    assert_eq!(
        perforated,
        ["CUT", "CREASE", "PERFORATION", "DIMENSIONS", "LABELS"]
    );
}

fn pdf_names(packaging: &Packaging) -> Vec<String> {
    layer_names(&pdf(packaging))
        .into_iter()
        .map(str::to_owned)
        .collect()
}

/// A cubic Bézier curve's point at `t`.
fn bezier(([p0, p1, p2, p3], t): ([Point; 4], f64)) -> Point {
    let u = 1.0 - t;
    let mix = |a: f64, b: f64, c: f64, d: f64| {
        u * u * u * a + 3.0 * u * u * t * b + 3.0 * u * t * t * c + t * t * t * d
    };
    Point::new(mix(p0.x, p1.x, p2.x, p3.x), mix(p0.y, p1.y, p2.y, p3.y))
}

/// The curves of each path in a layer: each path's start, then its curves' points.
fn curves(commands: &str) -> Vec<Vec<[Point; 4]>> {
    let mut paths = Vec::new();
    let mut current = Point::ORIGIN;
    let mut path: Vec<[Point; 4]> = Vec::new();
    for line in commands.lines() {
        let values = numbers(line);
        match line.split_whitespace().last() {
            Some("m") => current = Point::new(values[0], values[1]),
            Some("c") => {
                let points = [
                    Point::new(values[0], values[1]),
                    Point::new(values[2], values[3]),
                    Point::new(values[4], values[5]),
                ];
                path.push([current, points[0], points[1], points[2]]);
                current = points[2];
            }
            Some("S") => paths.push(std::mem::take(&mut path)),
            _ => {}
        }
    }
    paths
}

#[test]
fn arcs_become_curves_on_the_true_circle() {
    let packaging = common::arc_packaging();
    let sheet = Sheet::of((&packaging, Spacing::DRAWING));
    let pdf = pdf(&packaging);
    let cuts = curves(layer((&pdf, "CUT")));
    let creases = curves(layer((&pdf, "CREASE")));
    // The disc and its mirrored copy: a full circle in 8 eighths, a quarter turn in 2.
    assert_eq!(cuts.iter().map(Vec::len).collect::<Vec<_>>(), [8, 8]);
    assert_eq!(creases.iter().map(Vec::len).collect::<Vec<_>>(), [2, 2]);

    for (index, piece) in sheet.pieces.iter().enumerate() {
        // The disc's centre, (30, 30) in its own drawing, where the sheet put it.
        let own = Bounds::of(
            packaging.insert.pieces[0]
                .outline
                .flatten(CURVE_TOLERANCE_MM),
        )
        .unwrap();
        let x = match piece.mirrored {
            true => own.max.x - 30.0,
            false => 30.0 - own.min.x,
        };
        let center = Point::new(
            piece.bounds.min.x + x,
            piece.bounds.min.y + 30.0 - own.min.y,
        );
        for (curves, radius) in [(&cuts[index], 30.0), (&creases[index], 20.0)] {
            for &curve in curves {
                for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
                    let off = bezier((curve, t)).distance_to(center) - radius;
                    assert!(off.abs() < 1e-3, "{} at t = {t}: {off} mm off", piece.name);
                }
            }
        }
    }
    // The crease runs clockwise on screen from the top on the insert, and the other way
    // on the mirrored copy: its first curve heads right, then left.
    let heading = |curves: &[[Point; 4]]| curves[0][3].x - curves[0][0].x;
    assert!(heading(&creases[0]) > 0.0);
    assert!(heading(&creases[1]) < 0.0);
}

/// Six jars in an E-flute grid, in a B-flute shipping box: two rows.
fn grid_shipping() -> Packaging {
    let design = Design {
        outer_box: Some(insert_core::BoxSpec {
            style: insert_core::BoxStyle::Shipping,
            material: insert_core::Material::starting_values(insert_core::Board::BFlute),
            inside_mm: None,
        }),
        ..common::grid_design()
    };
    design.generate().unwrap()
}

#[test]
fn whole_files_match_their_golden_copies() {
    let failures: Vec<String> = [
        ("export_cradle_mailer.pdf", cradle_mailer()),
        ("export_grid_shipping.pdf", grid_shipping()),
    ]
    .into_iter()
    .filter_map(|(file, packaging)| common::golden::check((file, &pdf(&packaging))).err())
    .collect();
    assert!(failures.is_empty(), "\n{}", failures.join("\n\n"));
}
