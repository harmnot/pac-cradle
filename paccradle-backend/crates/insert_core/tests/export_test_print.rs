//! M1-B4: the print-at-home test print (CLAUDE.md U3, §6.6). Every copy of every piece is
//! printed once: whole when it fits a page (turned if need be), otherwise tiled with a
//! 10 mm overlap. Each page carries only its own part of the lines, folds are numbered by
//! stage and go away from the printed side, the watermark sits over every page, the scale
//! bar matches the paper, and the file holds together. Two whole files are golden
//! (`tests/golden/export_test_print_cradle_mailer_a4.pdf`).
//!
//! Needs the `export` feature: `cargo test -p insert_core --features export`.
#![cfg(feature = "export")]
// The helpers below are test code too; clippy.toml only exempts `#[test]` functions.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::collections::HashMap;

use common::pdf::{check_structure, numbers, streams};
use insert_core::export::{Joins, Paper, Polyline, Sheet, Spacing, TestItem, TestPrint, View};
use insert_core::{
    Board, Bounds, BoxSpec, BoxStyle, Design, FoldDirection, Insert, Line, Material, Packaging,
    Panel, Piece, Point, Polygon,
};

/// The drawing window on every page: inside a 10 mm margin, above a 10 mm footer.
const MARGIN_MM: f64 = 10.0;
const FOOTER_MM: f64 = 10.0;
const OVERLAP_MM: f64 = 10.0;
const GAP_MM: f64 = 6.0;
const LABEL_MM: f64 = 6.0;
const TILE_PAD_MM: f64 = 3.0;
const FOLD_MARK_RADIUS_MM: f64 = 2.4;
const PT_PER_MM: f64 = 72.0 / 25.4;

/// How each kind of line is drawn: the style line that starts its commands.
const LINE_STYLES: [&str; 3] = [
    "0 G 0.3 w [] 0 d",
    "0.1 0.25 0.8 RG 0.25 w [2.5 1.5] 0 d",
    "0 G 0.25 w [0.2 1.3] 0 d",
];

/// The engine's sample: two bottles on a chipboard cradle in an E-flute mailer.
fn cradle_mailer() -> Packaging {
    let design: Design = serde_json::from_str(include_str!("fixtures/cradle_mailer.json")).unwrap();
    design.generate().unwrap()
}

/// Six jars in an E-flute grid (one long divider, two short), in a B-flute shipping box.
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

/// A gloss and a compact, 30 mm tall, on a paperboard card in a fixed B-flute shipping
/// box 40 mm high.
fn card_shipping() -> Packaging {
    let mut design = common::card_design();
    for product in &mut design.products {
        product.height_mm = 30.0;
    }
    design.generate().unwrap()
}

/// Every sample on both papers.
fn samples() -> Vec<(&'static str, Packaging, Paper)> {
    let packagings = [
        ("cradle_mailer", cradle_mailer()),
        ("grid_shipping", grid_shipping()),
        ("card_shipping", card_shipping()),
    ];
    packagings
        .into_iter()
        .flat_map(|(name, packaging)| {
            [Paper::A4, Paper::Letter].map(|paper| (name, packaging.clone(), paper))
        })
        .collect()
}

/// An insert of plain rectangles, one piece each, cut once, no box.
fn rectangles(sizes: &[(f64, f64)]) -> Packaging {
    let pieces = sizes
        .iter()
        .map(|&(length, width)| {
            let outline = Polygon(vec![
                Point::new(0.0, 0.0),
                Point::new(length, 0.0),
                Point::new(length, width),
                Point::new(0.0, width),
            ]);
            Piece {
                name: format!("{length} x {width}"),
                quantity: 1,
                outline: outline.to_path().unwrap(),
                holes: Vec::new(),
                creases: Vec::new(),
                perforations: Vec::new(),
                panels: vec![Panel {
                    outline,
                    holes: Vec::new(),
                }],
                hinges: Vec::new(),
            }
        })
        .collect();
    Packaging {
        insert: Insert {
            pieces,
            length_mm: 0.0,
            width_mm: 0.0,
            height_mm: 1.0,
            placements: Vec::new(),
            mounts: Vec::new(),
        },
        outer_box: None,
    }
}

fn window(paper: Paper) -> Bounds {
    let (width, height) = paper.size_mm();
    Bounds {
        min: Point::new(MARGIN_MM, MARGIN_MM),
        max: Point::new(width - MARGIN_MM, height - MARGIN_MM - FOOTER_MM),
    }
}

/// Where item `item` is shown: each page index with the view.
fn views_of((print, item): (&TestPrint, usize)) -> Vec<(usize, View)> {
    print
        .pages
        .iter()
        .enumerate()
        .flat_map(|(index, page)| {
            page.views
                .iter()
                .filter(move |view| view.item == item)
                .map(move |view| (index, *view))
        })
        .collect()
}

fn pdf((packaging, paper): (&Packaging, Paper)) -> String {
    String::from_utf8(packaging.to_test_print(paper)).expect("the PDF is plain ASCII")
}

#[test]
fn every_copy_of_every_piece_is_printed_once() {
    for (name, packaging, paper) in samples() {
        let print = TestPrint::of((&packaging, paper));
        let sheet = Sheet::of((&packaging, Spacing::CUT_FILE));

        // The sheet's pieces in order, each copy after the one before.
        let expected: Vec<(String, u32)> = sheet
            .pieces
            .iter()
            .flat_map(|piece| (1..=piece.quantity).map(|copy| (piece.label(), copy)))
            .collect();
        let got: Vec<(String, u32)> = print
            .items
            .iter()
            .map(|item| (item.piece_label.clone(), item.copy))
            .collect();
        assert_eq!(got, expected, "{name}");

        for (index, item) in print.items.iter().enumerate() {
            let views = views_of((&print, index));
            let tiles: Vec<_> = views
                .iter()
                .filter_map(|(page, _)| print.pages[*page].tile)
                .collect();
            match tiles.first() {
                // Whole: one view, on one page.
                None => assert_eq!(views.len(), 1, "{name}: {}", item.label()),
                // Tiled: one page per tile, each showing nothing else.
                // Tiled: one page per tile it reaches, row by row, each showing nothing
                // else.
                Some(tile) => {
                    assert!(views.len() <= tile.columns * tile.rows, "{name}");
                    assert_eq!(tiles.len(), views.len(), "{name}");
                    let order: Vec<(usize, usize)> =
                        tiles.iter().map(|tile| (tile.row, tile.column)).collect();
                    assert!(order.is_sorted(), "{name}: {order:?}");
                    for (page, _) in &views {
                        assert_eq!(print.pages[*page].views.len(), 1, "{name}");
                    }
                }
            }
        }
    }
    // The grid's two short dividers are told apart.
    let print = TestPrint::of((&grid_shipping(), Paper::A4));
    let labels: Vec<String> = print.items.iter().map(|item| item.label()).collect();
    assert!(
        labels.contains(&"Insert: short divider (2 of 2)".to_owned()),
        "{labels:?}"
    );
    assert!(
        labels.contains(&"Insert: long divider".to_owned()),
        "{labels:?}"
    );
}

#[test]
fn whole_pieces_sit_apart_inside_the_window_with_their_labels() {
    for (name, packaging, paper) in samples() {
        let print = TestPrint::of((&packaging, paper));
        let window = window(paper).expanded(1e-9);
        for page in print.pages.iter().filter(|page| page.tile.is_none()) {
            // Each piece's box on the page, its label's band included.
            let boxes: Vec<Bounds> = page
                .views
                .iter()
                .map(|view| {
                    let item = &print.items[view.item];
                    let corner = view.to_page(Point::ORIGIN);
                    Bounds {
                        min: Point::new(corner.x, corner.y - LABEL_MM),
                        max: Point::new(corner.x + item.length_mm, corner.y + item.width_mm),
                    }
                })
                .collect();
            for (index, bounds) in boxes.iter().enumerate() {
                assert!(window.encloses(*bounds), "{name}: {bounds:?}");
                for other in &boxes[index + 1..] {
                    assert!(bounds.gap_to(*other) >= GAP_MM - 1e-9, "{name}");
                }
            }
        }
    }
}

/// True when some of `item`'s lines run through `window`, sampled every 0.5 mm.
fn crosses((item, window): (&TestItem, Bounds)) -> bool {
    let lines = item
        .cuts
        .iter()
        .chain(&item.creases)
        .chain(&item.perforations);
    lines.flat_map(segments).any(|(from, to)| {
        let steps = (from.distance_to(to) / 0.5).ceil().max(1.0) as usize;
        (0..=steps).any(|step| {
            let t = step as f64 / steps as f64;
            let point = Point::new(from.x + t * (to.x - from.x), from.y + t * (to.y - from.y));
            point.x >= window.min.x
                && point.x <= window.max.x
                && point.y >= window.min.y
                && point.y <= window.max.y
        })
    })
}

/// A line's straight segments, the closing one included.
fn segments(line: &Polyline) -> Vec<(Point, Point)> {
    let closing = line
        .closed
        .then(|| (*line.points.last().unwrap(), line.points[0]));
    line.points
        .windows(2)
        .map(|pair| (pair[0], pair[1]))
        .chain(closing)
        .collect()
}

#[test]
fn bigger_pieces_are_tiled_with_a_10_mm_overlap_and_no_spare_tiles() {
    let (mut tiled, mut left_out) = (0, 0);
    for (name, packaging, paper) in samples() {
        let print = TestPrint::of((&packaging, paper));
        let room = window(paper);
        let step = (room.length_mm() - OVERLAP_MM, room.width_mm() - OVERLAP_MM);
        for (index, item) in print.items.iter().enumerate() {
            let views = views_of((&print, index));
            let Some(first) = print.pages[views[0].0].tile else {
                continue;
            };
            tiled += 1;
            // Every tile's window on the grid, printed or not, from the first one's.
            let first_window = views[0].1.window;
            let window_of = |(column, row): (usize, usize)| {
                let min = Point::new(
                    first_window.min.x + (column as f64 - first.column as f64) * step.0,
                    first_window.min.y + (row as f64 - first.row as f64) * step.1,
                );
                Bounds {
                    min,
                    max: Point::new(min.x + room.length_mm(), min.y + room.width_mm()),
                }
            };
            let printed: HashMap<(usize, usize), View> = views
                .iter()
                .map(|(page, view)| {
                    let tile = print.pages[*page].tile.unwrap();
                    ((tile.column, tile.row), *view)
                })
                .collect();
            for (page, view) in &views {
                let tile = print.pages[*page].tile.unwrap();
                // Each the page's whole drawing window, on the grid, 10 mm over the next.
                assert_eq!(view.at, room.min, "{name}");
                let expected = window_of((tile.column, tile.row));
                assert!(view.window.min.distance_to(expected.min) < 1e-9, "{name}");
                assert!(view.window.max.distance_to(expected.max) < 1e-9, "{name}");
                let overlap = view.window.max.x - window_of((tile.column + 1, tile.row)).min.x;
                assert!((overlap - OVERLAP_MM).abs() < 1e-9, "{name}: {overlap}");
                // It joins exactly the printed tiles beside it.
                let has = |(column, row): (Option<usize>, Option<usize>)| match (column, row) {
                    (Some(column), Some(row)) => printed.contains_key(&(column, row)),
                    _ => false,
                };
                let (column, row) = (tile.column, tile.row);
                let joins = Joins {
                    left: has((column.checked_sub(1), Some(row))),
                    right: has((Some(column + 1), Some(row))),
                    top: has((Some(column), row.checked_sub(1))),
                    bottom: has((Some(column), Some(row + 1))),
                };
                assert_eq!(tile.joins, joins, "{name}: {}", tile.name());
            }
            // Tiles left out don't reach the piece: no line through them, and their
            // middle is outside the outline.
            for row in 0..first.rows {
                for column in
                    (0..first.columns).filter(|&column| !printed.contains_key(&(column, row)))
                {
                    left_out += 1;
                    let missing = window_of((column, row));
                    assert!(
                        !crosses((item, missing)),
                        "{name}: a line in a missing tile"
                    );
                    let outline = Polygon(item.cuts[0].points.clone());
                    assert!(!outline.contains(missing.center()), "{name}");
                }
            }
            // Together they cover the piece and its pad, centred on it.
            let covered = Bounds {
                min: window_of((0, 0)).min,
                max: window_of((first.columns - 1, first.rows - 1)).max,
            };
            let padded = Bounds::from_size((item.length_mm, item.width_mm)).expanded(TILE_PAD_MM);
            assert!(covered.encloses(padded), "{name}: {covered:?}");
            let centre = covered.center();
            assert!((centre.x - item.length_mm / 2.0).abs() < 1e-9);
            assert!((centre.y - item.width_mm / 2.0).abs() < 1e-9);
            // One tile fewer either way wouldn't do.
            if first.columns > 1 {
                let short = room.length_mm() + (first.columns - 2) as f64 * step.0;
                assert!(short < padded.length_mm(), "{name}: spare column");
            }
            if first.rows > 1 {
                let short = room.width_mm() + (first.rows - 2) as f64 * step.1;
                assert!(short < padded.width_mm(), "{name}: spare row");
            }
        }
    }
    assert!(tiled >= 4, "the samples should tile their boxes: {tiled}");
    assert!(left_out >= 1, "a box's empty corner should be left out");
}

#[test]
fn a_piece_turns_when_that_fits_the_paper_better() {
    // Too long for an A4 page's 190 mm, but fits on it upright; then one that takes
    // 3 pages as drawn and 2 turned; then one that fits as drawn.
    let packaging = rectangles(&[(240.0, 150.0), (400.0, 180.0), (150.0, 100.0)]);
    let print = TestPrint::of((&packaging, Paper::A4));
    let turned: Vec<bool> = print.items.iter().map(|item| item.turned).collect();
    assert_eq!(turned, [true, true, false]);
    let pages = |item: usize| views_of((&print, item)).len();
    assert_eq!((pages(0), pages(1), pages(2)), (1, 2, 1));
    assert!(print.pages[views_of((&print, 0))[0].0].tile.is_none());

    // A strip that fits as drawn turns to share the row beside one that had to turn,
    // instead of starting a page of its own.
    let strips = TestPrint::of((&rectangles(&[(240.0, 60.0), (150.0, 60.0)]), Paper::A4));
    let turned: Vec<bool> = strips.items.iter().map(|item| item.turned).collect();
    assert_eq!(turned, [true, true]);
    assert_eq!(strips.pages.len(), 1);
    let corners: Vec<Point> = strips.pages[0]
        .views
        .iter()
        .map(|view| view.to_page(Point::ORIGIN))
        .collect();
    assert_eq!(corners[0].y, corners[1].y, "one row");
    assert!((corners[1].x - corners[0].x - (60.0 + GAP_MM)).abs() < 1e-9);

    // Turned a quarter turn clockwise: the sheet's top edge is now on the right.
    let item = &print.items[0];
    assert_eq!((item.length_mm, item.width_mm), (150.0, 240.0));
    let outline = &item.cuts[0].points;
    assert_eq!(
        outline,
        &[
            Point::new(150.0, 0.0),
            Point::new(150.0, 240.0),
            Point::new(0.0, 240.0),
            Point::new(0.0, 0.0),
        ]
    );
}

/// The points of every cut, crease and perforation line drawn on a page.
fn line_points(content: &str) -> Vec<Point> {
    let mut points = Vec::new();
    let mut in_lines = false;
    for line in content.lines() {
        if LINE_STYLES.contains(&line) {
            in_lines = true;
            continue;
        }
        let ends_path = matches!(line, "S" | "h S");
        let is_point = line.ends_with(" m") || line.ends_with(" l");
        match (in_lines, is_point, ends_path) {
            (true, true, _) => {
                let values = numbers(line);
                points.push(Point::new(values[0], values[1]));
            }
            (true, false, true) => {}
            _ => in_lines = false,
        }
    }
    points
}

#[test]
fn each_page_carries_only_its_own_part_of_the_lines() {
    for (name, packaging, paper) in samples() {
        let print = TestPrint::of((&packaging, paper));
        let pdf = pdf((&packaging, paper));
        let pages = streams(&pdf);
        let drawings = &pages[pages.len() - print.pages.len()..];
        let window = window(paper).expanded(1e-6);
        let mut drawn = 0;
        for (index, content) in drawings.iter().enumerate() {
            let points = line_points(content);
            assert!(!points.is_empty(), "{name}: page {index} has no lines");
            for point in &points {
                assert!(
                    point.x >= window.min.x
                        && point.x <= window.max.x
                        && point.y >= window.min.y
                        && point.y <= window.max.y,
                    "{name}: {point:?} outside the window on drawing page {index}"
                );
            }
            drawn += points.len();
        }
        // The instructions draw no cut lines.
        for content in &pages[..pages.len() - print.pages.len()] {
            assert!(line_points(content).is_empty(), "{name}");
        }
        assert!(drawn > 0);
    }
}

/// The shortest distance from `point` to any segment of `lines`.
fn distance_to_lines((point, lines): (Point, &[Polyline])) -> f64 {
    lines
        .iter()
        .flat_map(segments)
        .map(|(from, to)| Line { from, to }.distance_to(point))
        .fold(f64::INFINITY, f64::min)
}

#[test]
fn folds_are_numbered_by_stage_and_go_away_from_the_printed_side() {
    let packaging = cradle_mailer();
    let print = TestPrint::of((&packaging, Paper::A4));
    let pieces: Vec<&Piece> = packaging
        .insert
        .pieces
        .iter()
        .chain(
            packaging
                .outer_box
                .iter()
                .flat_map(|outer_box| &outer_box.pieces),
        )
        .collect();
    assert_eq!(print.items.len(), pieces.len());
    for (item, piece) in print.items.iter().zip(&pieces) {
        assert_eq!(item.folds.len(), piece.hinges.len(), "{}", item.label());
        let mut stages: Vec<u32> = piece.hinges.iter().map(|hinge| hinge.stage).collect();
        stages.sort_unstable();
        stages.dedup();
        for (fold, hinge) in item.folds.iter().zip(&piece.hinges) {
            let rank = stages
                .iter()
                .position(|&stage| stage == hinge.stage)
                .unwrap();
            assert_eq!(fold.step as usize, rank + 1);
            // The cradle folds down from its deck; the mailer is drawn from the inside
            // with valley folds, and prints from the outside.
            assert_eq!(fold.direction, FoldDirection::Mountain, "{}", item.label());
            // The fold, turned and mirrored with the lines, lies on a crease.
            for end in [fold.axis.from, fold.axis.to] {
                let off = distance_to_lines((end, &item.creases));
                assert!(
                    off < 0.02,
                    "{}: fold end {off} mm off its crease",
                    item.label()
                );
            }
        }
    }
    let steps = |index: usize| {
        let mut steps: Vec<u32> = print.items[index]
            .folds
            .iter()
            .map(|fold| fold.step)
            .collect();
        steps.sort_unstable();
        steps.dedup();
        steps
    };
    assert_eq!(steps(0), [1, 2, 3], "the cradle's wall, foot and riser");
    assert_eq!(steps(1).len(), 7, "the mailer's seven stages");

    // The instructions give each step; every fold gets a numbered circle on a page.
    let pdf = pdf((&packaging, Paper::A4));
    let pages = streams(&pdf);
    let guide = pages[..pages.len() - print.pages.len()].concat();
    assert!(guide.contains("(Insert: cradle) Tj"));
    assert!(guide.contains("(Box: mailer) Tj"));
    assert!(guide.contains("away from the printed side"));
    assert!(!guide.contains("towards the printed side"));
    assert!(guide.contains("(Fold the 2 creases marked 1 by 90 degrees, away from the printed"));
    let drawings = pages[pages.len() - print.pages.len()..].concat();
    let marks = drawings.matches("q 1 g 0 G 0.2 w [] 0 d\n").count();
    let folds: usize = print.items.iter().map(|item| item.folds.len()).sum();
    assert!(marks >= folds, "{marks} marks for {folds} folds");
    assert!(
        !drawings.contains("q 0 g 0 G 0.2 w [] 0 d\n"),
        "no folds towards the print"
    );
}

/// The fold marks' centres on a page: each circle starts at its rightmost point.
fn mark_centres(content: &str) -> Vec<Point> {
    let mut lines = content.lines();
    let mut centres = Vec::new();
    while let Some(line) = lines.next() {
        if line.starts_with("q ") && line.ends_with(" g 0 G 0.2 w [] 0 d") {
            let start = numbers(lines.next().unwrap());
            centres.push(Point::new(start[0] - FOLD_MARK_RADIUS_MM, start[1]));
        }
    }
    centres
}

#[test]
fn fold_marks_keep_clear_of_each_other_and_of_the_seams() {
    for (name, packaging, paper) in samples() {
        let print = TestPrint::of((&packaging, paper));
        let pdf = pdf((&packaging, paper));
        let pages = streams(&pdf);
        let mut marks = 0;
        let drawings = &pages[pages.len() - print.pages.len()..];
        for (page, content) in print.pages.iter().zip(drawings) {
            let centres = mark_centres(content);
            marks += centres.len();
            // On a tile, out of the overlaps where it joins its neighbours, unless its
            // whole crease lies in one (then no spot along it would do).
            if let Some(tile) = page.tile {
                let view = page.views[0];
                let item = &print.items[view.item];
                let room = window(paper);
                let half = OVERLAP_MM / 2.0;
                let reach = half + FOLD_MARK_RADIUS_MM - 1e-9;
                // Each joined seam: true for one running down the page, and where.
                let seams = [
                    (tile.joins.left, true, room.min.x + half),
                    (tile.joins.right, true, room.max.x - half),
                    (tile.joins.top, false, room.min.y + half),
                    (tile.joins.bottom, false, room.max.y - half),
                ];
                for centre in &centres {
                    let spot = Point::new(
                        centre.x - view.at.x + view.window.min.x,
                        centre.y - view.at.y + view.window.min.y,
                    );
                    let fold = item
                        .folds
                        .iter()
                        .find(|fold| fold.axis.distance_to(spot) < 1e-6)
                        .expect("a mark sits on its crease");
                    for (_, down, at) in seams.into_iter().filter(|seam| seam.0) {
                        let across = |point: Point| match down {
                            true => view.to_page(point).x - at,
                            false => view.to_page(point).y - at,
                        };
                        let off = match down {
                            true => centre.x - at,
                            false => centre.y - at,
                        };
                        let crease_inside = across(fold.axis.from).abs() < reach
                            && across(fold.axis.to).abs() < reach;
                        assert!(
                            off.abs() >= reach || crease_inside,
                            "{name}: mark at {centre:?} in an overlap"
                        );
                    }
                }
            }
            for (index, centre) in centres.iter().enumerate() {
                for other in &centres[index + 1..] {
                    let apart = centre.distance_to(*other);
                    assert!(
                        apart >= 2.0 * FOLD_MARK_RADIUS_MM,
                        "{name}: marks {apart} mm apart"
                    );
                }
            }
        }
        let folds: usize = print.items.iter().map(|item| item.folds.len()).sum();
        assert!(marks >= folds, "{name}: {marks} marks for {folds} folds");
    }
}

#[test]
fn the_instructions_hold_only_the_sections_that_apply() {
    let guide = |(packaging, paper): (&Packaging, Paper)| {
        let print = TestPrint::of((packaging, paper));
        let pdf = pdf((packaging, paper));
        let pages = streams(&pdf);
        pages[..pages.len() - print.pages.len()].concat()
    };
    // The grid: tiled box, folds, dividers to slot together.
    let grid = guide((&grid_shipping(), Paper::Letter));
    for heading in [
        "(1. Print) Tj",
        "(2. Join the pages) Tj",
        "(3. Cut) Tj",
        "(4. Fold) Tj",
        "(5. Slot the dividers together) Tj",
        "(6. Check the fit) Tj",
    ] {
        assert!(grid.contains(heading), "{heading}");
    }
    assert!(grid.contains("(Box: shipping box: "), "a map for the box");
    assert!(grid.contains("(A1) Tj"));
    assert!(grid.contains("must measure 2 in"));

    // The cradle has no dividers.
    let cradle = guide((&cradle_mailer(), Paper::A4));
    assert!(cradle.contains("must measure 50 mm"));
    assert!(!cradle.contains("Slot the dividers"));

    // Small flat pieces: nothing to join or fold.
    let flat = guide((&rectangles(&[(100.0, 60.0)]), Paper::A4));
    assert!(flat.contains("(1. Print) Tj") && flat.contains("(2. Cut) Tj"));
    assert!(flat.contains("(3. Check the fit) Tj"));
    assert!(!flat.contains("Join the pages") && !flat.contains("Fold"));
}

#[test]
fn the_watermark_runs_over_every_page_after_the_drawing() {
    for (name, packaging, paper) in samples() {
        let pdf = pdf((&packaging, paper));
        assert!(pdf.contains("<< /Type /ExtGState /ca 0.2 /CA 0.2 >>"));
        let pages = streams(&pdf);
        let mut first_rows = Vec::new();
        for (index, content) in pages.iter().enumerate() {
            let watermark = content.find("q /GS1 gs ").expect("a watermark");
            // Drawn last, over the lines, labels, marks and footer.
            assert!(
                content[watermark..].ends_with("Q\nQ\n"),
                "{name}: page {index}"
            );
            assert!(
                !content[watermark..].contains(" S\n"),
                "{name}: lines after it"
            );
            let rows = content[watermark..]
                .matches("(PACCRADLE TEST PRINT")
                .count();
            assert!(rows >= 15, "{name}: page {index} has {rows} rows");
            let first = content[watermark..]
                .lines()
                .find(|line| line.starts_with("BT /F2"))
                .unwrap()
                .to_owned();
            first_rows.push(first);
        }
        // Each page shifts its rows its own way.
        first_rows.sort();
        first_rows.dedup();
        assert_eq!(first_rows.len(), pages.len(), "{name}");
    }
}

#[test]
fn the_file_holds_together_at_true_size() {
    for (name, packaging, paper) in samples() {
        let print = TestPrint::of((&packaging, paper));
        let pdf = pdf((&packaging, paper));
        let objects = check_structure(&pdf);
        let pages = streams(&pdf);
        // The catalog, the page list, two fonts, the watermark setting and the info,
        // then a page and its content each.
        assert_eq!(objects, 6 + 2 * pages.len(), "{name}");
        assert!(
            pages.len() > print.pages.len(),
            "{name}: instructions first"
        );
        assert!(pdf.contains(&format!("/Count {} >>", pages.len())));

        // Every page is the paper, upright, at true size.
        let (width, height) = paper.size_mm();
        for (at, _) in pdf.match_indices("/MediaBox [") {
            let rest = &pdf[at + "/MediaBox [".len()..];
            let size = numbers(&rest[..rest.find(']').unwrap()]);
            assert!(
                (size[2] - width * PT_PER_MM).abs() < 1e-3,
                "{name}: {size:?}"
            );
            assert!(
                (size[3] - height * PT_PER_MM).abs() < 1e-3,
                "{name}: {size:?}"
            );
        }
        assert_eq!(pdf.matches("/MediaBox [").count(), pages.len());
        assert!(pdf.contains("/ViewerPreferences << /PrintScaling /None >>"));
        // No layers: not a production file.
        assert!(!pdf.contains("/OCProperties") && !pdf.contains("/OC /"));

        for (index, content) in pages.iter().enumerate() {
            let footer = format!("(Page {} of {} - ", index + 1, pages.len());
            assert!(content.contains(&footer), "{name}: {footer}");
        }
    }
}

#[test]
fn the_scale_bar_matches_the_paper() {
    for (paper, label, length) in [(Paper::A4, "50 mm", 50.0), (Paper::Letter, "2 in", 50.8)] {
        let pdf = pdf((&cradle_mailer(), paper));
        let right = window(paper).max.x;
        let outline = format!(
            "{} {} {} 2 re S",
            round(right - length),
            round(paper.size_mm().1 - MARGIN_MM - 2.5 - 2.0),
            round(length)
        );
        for content in streams(&pdf) {
            assert!(content.contains(&format!("(check: this bar is {label}) Tj")));
            assert!(content.contains(&outline), "{outline}");
        }
    }
}

/// A number as the file writes it.
fn round(value: f64) -> String {
    let text = format!("{value:.4}");
    text.trim_end_matches('0').trim_end_matches('.').to_owned()
}

#[test]
fn whole_files_match_their_golden_copies() {
    let failures: Vec<String> = [
        (
            "export_test_print_cradle_mailer_a4.pdf",
            cradle_mailer(),
            Paper::A4,
        ),
        (
            "export_test_print_grid_shipping_letter.pdf",
            grid_shipping(),
            Paper::Letter,
        ),
    ]
    .into_iter()
    .filter_map(|(file, packaging, paper)| {
        common::golden::check((file, &pdf((&packaging, paper)))).err()
    })
    .collect();
    assert!(failures.is_empty(), "\n{}", failures.join("\n\n"));
}

/// The arc sample (a disc with a curved crease and a perforation) prints with straight
/// lines only: the preview's geometry, not the production file's arcs.
#[test]
fn lines_are_the_previews_straight_segments() {
    let packaging = common::arc_packaging();
    let print = TestPrint::of((&packaging, Paper::A4));
    let pdf = pdf((&packaging, Paper::A4));
    let pages = streams(&pdf);
    for content in &pages[pages.len() - print.pages.len()..] {
        let mut in_lines = false;
        for line in content.lines() {
            in_lines = match LINE_STYLES.contains(&line) {
                true => true,
                false => {
                    in_lines
                        && (line.ends_with(" m") || line.ends_with(" l") || line.ends_with('S'))
                }
            };
            assert!(!(in_lines && line.ends_with(" c")), "a curve in the lines");
        }
    }
    // The disc's outline is the preview's: its corners, one per straight segment.
    let preview = packaging.rounded_for_preview();
    let corners = preview.insert.pieces[0].outline.segments.len() + 1;
    assert_eq!(print.items[0].cuts[0].points.len(), corners);
    // Its perforation is drawn dotted.
    assert!(pdf.contains(&format!("{}\n", LINE_STYLES[2])));
}
