//! M1-B5: the free watermarked PNG preview (CLAUDE.md §3 step 10, §7.4). The file is a PNG
//! of the laid-out size; products show their fit colours in the top view; pieces show
//! their cut lines and creases where the layout puts them; the watermark reaches every
//! part of the picture; big designs stay within the size limit; the same design draws
//! the same picture. Two pictures are golden, compared pixel by pixel within a small
//! tolerance (`tests/golden/preview_cradle_mailer.png`).
//!
//! Needs the `export` feature: `cargo test -p insert_core --features export`.
#![cfg(feature = "export")]
// The helpers below are test code too; clippy.toml only exempts `#[test]` functions.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::path::PathBuf;

use insert_core::export::{Block, BlockKind, Preview, Sheet, Spacing};
use insert_core::{
    Board, BoxSpec, BoxStyle, Design, FitColor, Insert, Material, Packaging, Panel, Piece, Point,
    Polygon,
};
use tiny_skia::Pixmap;

/// The picture's background (label cream) and the watermark over it (espresso at 13 %).
const BACKGROUND: [u8; 3] = [0xfa, 0xf6, 0xef];
const ESPRESSO: [u8; 3] = [0x2b, 0x22, 0x19];
const WATERMARK_ALPHA: f64 = 0.13;
const MAX_SIDE_PX: u32 = 4000;

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

/// A gloss and a compact, 30 mm tall, on a paperboard card in a fixed shipping box.
fn card_shipping() -> Design {
    let mut design = common::card_design();
    for product in &mut design.products {
        product.height_mm = 30.0;
    }
    design
}

fn picture(design: &Design) -> (Preview, Pixmap, Vec<u8>) {
    let packaging = design.generate().unwrap();
    let preview = Preview::of((&packaging, design)).unwrap();
    let png = packaging.to_png(design).unwrap();
    let pixmap = Pixmap::decode_png(&png).expect("a PNG file");
    (preview, pixmap, png)
}

/// The colour at a pixel, without alpha (the picture is opaque).
fn rgb((pixmap, at): (&Pixmap, Point)) -> Option<[u8; 3]> {
    let (x, y) = (at.x.round(), at.y.round());
    if x < 0.0 || y < 0.0 {
        return None;
    }
    let pixel = pixmap.pixel(x as u32, y as u32)?.demultiply();
    Some([pixel.red(), pixel.green(), pixel.blue()])
}

/// Every pixel within `radius` px of `at`.
fn patch((pixmap, at, radius): (&Pixmap, Point, i32)) -> Vec<[u8; 3]> {
    (-radius..=radius)
        .flat_map(|dy| (-radius..=radius).map(move |dx| (dx, dy)))
        .filter_map(|(dx, dy)| {
            rgb((
                pixmap,
                Point::new(at.x + f64::from(dx), at.y + f64::from(dy)),
            ))
        })
        .collect()
}

fn top_view(preview: &Preview) -> &Block {
    preview
        .blocks
        .iter()
        .find(|block| block.kind == BlockKind::TopView)
        .unwrap()
}

#[test]
fn the_picture_is_a_png_of_the_laid_out_size() {
    for design in [cradle_mailer(), grid_shipping(), card_shipping()] {
        let (preview, pixmap, png) = picture(&design);
        assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
        assert_eq!(
            (pixmap.width(), pixmap.height()),
            (preview.width_px, preview.height_px)
        );
        assert!(preview.width_px.max(preview.height_px) <= MAX_SIDE_PX);
        assert!(preview.width_px >= 1000, "{}", preview.width_px);
        // The top view first, then each production-sheet piece once.
        let packaging = design.generate().unwrap();
        let sheet = Sheet::of((&packaging, Spacing::CUT_FILE));
        let kinds: Vec<BlockKind> = preview.blocks.iter().map(|block| block.kind).collect();
        let expected: Vec<BlockKind> = std::iter::once(BlockKind::TopView)
            .chain((0..sheet.pieces.len()).map(BlockKind::Piece))
            .collect();
        assert_eq!(kinds, expected);
        // Every drawing inside the picture, its label above it.
        for block in &preview.blocks {
            let corner = preview.to_px((block, block.origin_mm));
            let far = preview.to_px((
                block,
                Point::new(
                    block.origin_mm.x + block.size_mm.0,
                    block.origin_mm.y + block.size_mm.1,
                ),
            ));
            assert!(corner.x >= 0.0 && corner.y >= 0.0);
            assert!(far.x <= f64::from(preview.width_px) && far.y <= f64::from(preview.height_px));
            assert!(block.label.at.y < corner.y, "{}", block.label.text);
        }
    }
}

#[test]
fn the_top_view_is_drawn_larger_when_the_pieces_scale_leaves_it_small() {
    for design in [cradle_mailer(), grid_shipping()] {
        let (preview, _, _) = picture(&design);
        let view = top_view(&preview);
        // About 720 px along its longer side, more than the pieces' scale, and labelled so.
        let longer = view.size_mm.0.max(view.size_mm.1) * view.scale;
        assert!((longer - 720.0).abs() < 1e-6, "{longer}");
        assert!(view.scale > preview.scale);
        assert!(
            view.label.text.ends_with("(enlarged)"),
            "{}",
            view.label.text
        );
        for block in preview
            .blocks
            .iter()
            .filter(|block| block.kind != BlockKind::TopView)
        {
            assert_eq!(block.scale, preview.scale);
        }
    }
}

/// True when most of the colours lean towards `color`'s hue.
fn mostly((colors, color): (&[[u8; 3]], FitColor)) -> bool {
    let leaning = colors
        .iter()
        .filter(|&&[red, green, blue]| {
            let (red, green, blue) = (i32::from(red), i32::from(green), i32::from(blue));
            match color {
                FitColor::Green => green > red + 15 && green > blue + 15,
                FitColor::Yellow => red > blue + 40 && green > blue + 25 && red >= green,
                FitColor::Red => red > green + 40 && red > blue + 40,
            }
        })
        .count();
    leaning * 2 > colors.len()
}

/// Each product copy's colour in the top view, sampled round its centre.
fn product_colors(design: &Design) -> Vec<(FitColor, bool)> {
    let packaging = design.generate().unwrap();
    let fits = packaging.fits(design).unwrap();
    let (preview, pixmap, _) = picture(design);
    let view = top_view(&preview);
    fits.iter()
        .map(|fit| {
            let center = packaging.insert.placements[fit.placement].center_mm;
            let colors = patch((&pixmap, preview.to_px((view, center)), 3));
            (fit.color, mostly((&colors, fit.color)))
        })
        .collect()
}

#[test]
fn products_show_their_fit_colours() {
    // The cradle's and the grid's products fit with room: green.
    for design in [cradle_mailer(), grid_shipping()] {
        for (color, shown) in product_colors(&design) {
            assert_eq!(color, FitColor::Green);
            assert!(shown, "a green product isn't green in the picture");
        }
    }
    // The card sample pins its gloss off the card's edge (red); the compact is green.
    let colors = product_colors(&card_shipping());
    assert_eq!(
        colors,
        [(FitColor::Red, true), (FitColor::Green, true)],
        "each in its own colour"
    );
    // The card cut tighter than its board's suggested room: the compact turns yellow.
    let mut tight = card_shipping();
    tight.material.clearance_mm = 0.4;
    let colors = product_colors(&tight);
    assert_eq!(colors, [(FitColor::Red, true), (FitColor::Yellow, true)]);
    // The compact pushed onto the gloss: both red.
    let mut clash = card_shipping();
    if let insert_core::InsertSpec::CutoutCard(spec) = &mut clash.insert {
        spec.placements.push(insert_core::Placement {
            product_id: "compact".to_owned(),
            center_mm: Point::new(45.0, 60.0),
            rotation_deg: 0.0,
        });
    }
    let colors = product_colors(&clash);
    assert!(
        colors
            .iter()
            .all(|(color, shown)| *color == FitColor::Red && *shown),
        "{colors:?}"
    );
}

/// True when some pixel within `radius` of `at` is dark: a cut line.
fn dark_near((pixmap, at, radius): (&Pixmap, Point, i32)) -> bool {
    patch((pixmap, at, radius))
        .iter()
        .any(|&[red, green, blue]| u32::from(red) + u32::from(green) + u32::from(blue) < 300)
}

/// True when some pixel within `radius` of `at` leans to terracotta: a crease.
fn terracotta_near((pixmap, at, radius): (&Pixmap, Point, i32)) -> bool {
    patch((pixmap, at, radius))
        .iter()
        .any(|&[red, green, blue]| {
            let (red, green, blue) = (i32::from(red), i32::from(green), i32::from(blue));
            red > green + 40 && red > blue + 50 && red < 220
        })
}

#[test]
fn pieces_show_their_cut_lines_and_creases_where_the_layout_puts_them() {
    for design in [cradle_mailer(), grid_shipping(), card_shipping()] {
        let packaging = design.generate().unwrap();
        let sheet = Sheet::of((&packaging.rounded_for_preview(), Spacing::CUT_FILE));
        let (preview, pixmap, _) = picture(&design);
        for block in &preview.blocks {
            let BlockKind::Piece(index) = block.kind else {
                continue;
            };
            let piece = &sheet.pieces[index];
            // Every corner of the outline sits on a dark line.
            let outline = piece.cuts[0].flatten(0.05);
            let step = (outline.len() / 40).max(1);
            for point in outline.iter().step_by(step) {
                let at = preview.to_px((block, *point));
                assert!(
                    dark_near((&pixmap, at, 2)),
                    "{}: no cut line at {at:?}",
                    piece.title()
                );
            }
            // Most creases show terracotta dashes along them.
            for crease in &piece.creases {
                let points = crease.flatten(0.05);
                let (from, to) = (points[0], points[points.len() - 1]);
                let samples = 10;
                let hits = (1..samples)
                    .filter(|step| {
                        let t = f64::from(*step) / f64::from(samples);
                        let point =
                            Point::new(from.x + t * (to.x - from.x), from.y + t * (to.y - from.y));
                        terracotta_near((&pixmap, preview.to_px((block, point)), 4))
                    })
                    .count();
                assert!(
                    hits >= 4,
                    "{}: a crease with {hits} dashes found",
                    piece.title()
                );
            }
        }
    }
}

/// The background with the watermark over it.
fn watermark_color() -> [u8; 3] {
    let mut color = [0; 3];
    for channel in 0..3 {
        let (under, over) = (f64::from(BACKGROUND[channel]), f64::from(ESPRESSO[channel]));
        color[channel] = (under + (over - under) * WATERMARK_ALPHA).round() as u8;
    }
    color
}

#[test]
fn the_watermark_reaches_every_part_of_the_picture() {
    let watermark = watermark_color();
    let near = |[red, green, blue]: [u8; 3]| {
        [red, green, blue]
            .iter()
            .zip(watermark)
            .all(|(got, want)| got.abs_diff(want) <= 4)
    };
    for design in [cradle_mailer(), grid_shipping()] {
        let (_, pixmap, _) = picture(&design);
        // Every 400 px square holds watermark letters on plain background.
        let cell = 400;
        for top in (0..pixmap.height()).step_by(cell) {
            for left in (0..pixmap.width()).step_by(cell) {
                let found = (top..(top + cell as u32).min(pixmap.height()))
                    .step_by(2)
                    .flat_map(|y| {
                        (left..(left + cell as u32).min(pixmap.width()))
                            .step_by(2)
                            .map(move |x| (x, y))
                    })
                    .filter_map(|(x, y)| rgb((&pixmap, Point::new(f64::from(x), f64::from(y)))))
                    .any(near);
                assert!(found, "no watermark in the square at ({left}, {top})");
            }
        }
    }
}

#[test]
fn the_same_design_draws_the_same_picture() {
    let design = cradle_mailer();
    let (_, _, first) = picture(&design);
    let (_, _, second) = picture(&design);
    assert_eq!(first, second);
}

/// An insert of plain rectangles, one piece each, cut once, no box and no products.
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
                name: "sheet".to_owned(),
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
            length_mm: 100.0,
            width_mm: 100.0,
            height_mm: 1.0,
            placements: Vec::new(),
            mounts: Vec::new(),
        },
        outer_box: None,
    }
}

#[test]
fn big_designs_stay_within_the_size_limit_and_small_ones_stop_growing() {
    let design = common::grid_design();
    // Pieces metres long: the picture shrinks to fit.
    let huge = rectangles(&[(3000.0, 1500.0), (2500.0, 2000.0), (1800.0, 900.0)]);
    let preview = Preview::of((&huge, &design)).unwrap();
    assert!(
        preview.width_px.max(preview.height_px) <= MAX_SIDE_PX,
        "{preview:?}"
    );
    let png = huge.to_png(&design).unwrap();
    let pixmap = Pixmap::decode_png(&png).unwrap();
    assert_eq!(
        (pixmap.width(), pixmap.height()),
        (preview.width_px, preview.height_px)
    );
    // A tiny piece: at most 10 px per mm.
    let tiny = rectangles(&[(20.0, 10.0)]);
    let preview = Preview::of((&tiny, &design)).unwrap();
    assert!((preview.scale - 10.0).abs() < 1e-9, "{}", preview.scale);
}

/// Pixels whose colour differs from the golden picture's by more than this, in any
/// channel, count as changed. Smoothing can differ by a shade between processors.
const PIXEL_SLACK: u8 = 24;
/// At most this share of the pixels may change.
const CHANGED_SHARE: f64 = 0.001;

/// Compares `png` with the golden picture `tests/golden/<file>`, or writes it there when
/// `UPDATE_GOLDEN` is set. A difference writes the new picture to the target folder.
fn check_golden((file, png): (&str, &[u8])) -> Result<(), String> {
    let golden = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(file);
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::write(&golden, png).unwrap();
        return Ok(());
    }
    let new = Pixmap::decode_png(png).unwrap();
    let difference = match std::fs::read(&golden)
        .ok()
        .and_then(|bytes| Pixmap::decode_png(&bytes).ok())
    {
        None => Some("no golden picture".to_owned()),
        Some(old) if (old.width(), old.height()) != (new.width(), new.height()) => Some(format!(
            "size {} x {}, golden {} x {}",
            new.width(),
            new.height(),
            old.width(),
            old.height()
        )),
        Some(old) => {
            let changed = old
                .pixels()
                .iter()
                .zip(new.pixels())
                .filter(|(a, b)| {
                    let (a, b) = (a.demultiply(), b.demultiply());
                    a.red().abs_diff(b.red()) > PIXEL_SLACK
                        || a.green().abs_diff(b.green()) > PIXEL_SLACK
                        || a.blue().abs_diff(b.blue()) > PIXEL_SLACK
                })
                .count();
            let share = changed as f64 / new.pixels().len() as f64;
            (share > CHANGED_SHARE)
                .then(|| format!("{changed} pixels changed ({:.3} %)", share * 100.0))
        }
    };
    let Some(difference) = difference else {
        return Ok(());
    };
    let actual = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("golden");
    std::fs::create_dir_all(&actual).unwrap();
    std::fs::write(actual.join(file), png).unwrap();
    Err(format!(
        "{file}: {difference}.\n  new picture: {}",
        actual.join(file).display()
    ))
}

#[test]
fn pictures_match_their_golden_copies() {
    let failures: Vec<String> = [
        ("preview_cradle_mailer.png", cradle_mailer()),
        ("preview_grid_shipping.png", grid_shipping()),
    ]
    .into_iter()
    .filter_map(|(file, design)| check_golden((file, &picture(&design).2)).err())
    .collect();
    assert!(failures.is_empty(), "\n{}", failures.join("\n\n"));
}
