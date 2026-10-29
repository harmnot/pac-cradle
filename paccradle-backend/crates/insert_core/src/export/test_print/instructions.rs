//! The test print's first page or pages: how to print it, join the tiles, cut, fold and
//! check the fit. Sections appear only when they apply; the fold steps come from each
//! piece's hinges, and tile maps from the layout. English at launch (DEC-11).

use std::collections::BTreeMap;
use std::fmt::Write;

use super::pages::{FOLD_MARK_RADIUS_MM, counted, fold_mark};
use super::{TestItem, TestPrint, Tile};
use crate::check::Part;
use crate::export::LineKind;
use crate::export::num;
use crate::export::pdf::draw::{Font, Text, text_at, text_width_mm};
use crate::geometry::{Bounds, Point};
use crate::piece::FoldDirection;

/// Text sizes (the font size, in mm).
const TITLE_MM: f64 = 8.0;
const HEADING_MM: f64 = 4.5;
const BODY_MM: f64 = 3.4;
/// Lines of text sit this many font sizes apart.
const LEADING: f64 = 1.45;
const ABOVE_HEADING_MM: f64 = 4.0;
const ABOVE_SUBHEADING_MM: f64 = 1.5;
const AFTER_BLOCK_MM: f64 = 1.5;
/// Bullets and fold steps start their text this far in.
const BULLET_INDENT_MM: f64 = 5.0;
const FOLD_INDENT_MM: f64 = 8.0;
/// How far a fold step's circle sits above the first line's baseline.
const FOLD_MARK_RISE_MM: f64 = 1.2;
/// Tile maps: each tile a box up to this wide, shaped like the page's drawing window.
const MAP_TILE_MM: f64 = 12.0;
const MAP_NAME_MM: f64 = 3.5;
const MAP_LINE_MM: f64 = 0.2;
/// Helvetica's capitals stand about this share of the font size above the baseline,
/// halved: how far below the middle a centred name's baseline goes.
const CAP_HALF_HEIGHT: f64 = 0.36;

const INTRO: &str = "A paper prototype of your packaging at true size, to check the size \
    and the fit before you order. It is not a production file: the export has the files \
    your printer or cutter needs.";

/// One piece of the instructions, stacked down the page.
enum Block {
    Title(&'static str),
    Heading(String),
    Subheading(String),
    Paragraph(String),
    Bullet(String),
    /// A fold step: its numbered circle, then what to do.
    Fold {
        step: u32,
        direction: FoldDirection,
        text: String,
    },
    /// How a tiled piece's pages fit together: its printed tiles' names in a grid of
    /// `columns` × `rows`.
    Map {
        label: String,
        columns: usize,
        rows: usize,
        tiles: Vec<Tile>,
    },
}

/// The instructions' page bodies: blocks stacked down the drawing window, a new page when
/// the next one doesn't fit. A heading stays with the block after it.
pub(super) fn pages(print: &TestPrint) -> Vec<String> {
    let window = print.paper.window();
    let blocks = blocks(print);
    let mut pages = vec![String::new()];
    let mut y = window.min.y;
    for (index, block) in blocks.iter().enumerate() {
        let height = block.height(window);
        let keep = match (block, blocks.get(index + 1)) {
            (Block::Heading(_) | Block::Subheading(_), Some(next)) => height + next.height(window),
            _ => height,
        };
        if y + keep > window.max.y && y > window.min.y {
            pages.push(String::new());
            y = window.min.y;
        }
        if let Some(page) = pages.last_mut() {
            page.push_str(&block.draw((window, y)));
        }
        y += height;
    }
    pages
}

/// Every block, in order: title, intro, then the numbered sections that apply.
fn blocks(print: &TestPrint) -> Vec<Block> {
    let mut sections = vec![("Print", print_section(print))];
    let maps = maps(print);
    if !maps.is_empty() {
        sections.push(("Join the pages", join_section(maps)));
    }
    sections.push(("Cut", cut_section(print)));
    if print.items.iter().any(|item| !item.creases.is_empty()) {
        sections.push(("Fold", fold_section(print)));
    }
    if print.slotted_strips {
        sections.push(("Slot the dividers together", slot_section()));
    }
    sections.push(("Check the fit", fit_section()));

    let mut blocks = vec![
        Block::Title("Test print"),
        Block::Paragraph(INTRO.to_owned()),
    ];
    for (number, (heading, section)) in sections.into_iter().enumerate() {
        blocks.push(Block::Heading(format!("{}. {heading}", number + 1)));
        blocks.extend(section);
    }
    blocks
}

fn print_section(print: &TestPrint) -> Vec<Block> {
    vec![
        Block::Bullet(
            "Print every page at 100 % (actual size). Turn off \"fit to page\" and \
             \"shrink oversized pages\"."
                .to_owned(),
        ),
        Block::Bullet(format!(
            "Check the scale: the bar at the foot of each page must measure {}. If it \
             doesn't, the pieces won't match your product.",
            print.paper.scale_bar().label
        )),
        Block::Bullet(
            "Thick paper or thin card (160 to 300 g/m2) folds more like real board than copy \
             paper does."
                .to_owned(),
        ),
    ]
}

/// A map for each tiled item, from its pages (which come one after another).
fn maps(print: &TestPrint) -> Vec<Block> {
    let mut maps: Vec<(usize, Vec<Tile>)> = Vec::new();
    for page in &print.pages {
        let (Some(tile), Some(view)) = (page.tile, page.views.first()) else {
            continue;
        };
        match maps.last_mut() {
            Some((item, tiles)) if *item == view.item => tiles.push(tile),
            _ => maps.push((view.item, vec![tile])),
        }
    }
    maps.into_iter()
        .filter_map(|(item, tiles)| {
            let item = print.items.get(item)?;
            let first = *tiles.first()?;
            Some(Block::Map {
                label: format!(
                    "{}: {} in {} x {}",
                    item.label(),
                    counted((tiles.len(), "tile")),
                    counted((first.rows, "row")),
                    counted((first.columns, "column"))
                ),
                columns: first.columns,
                rows: first.rows,
                tiles,
            })
        })
        .collect()
}

fn join_section(maps: Vec<Block>) -> Vec<Block> {
    let mut blocks = vec![
        Block::Paragraph(
            "Pieces too big for one sheet are spread over several pages, called tiles. A \
             tile's name is in its footer: its row's letter and its column's number."
                .to_owned(),
        ),
        Block::Bullet(
            "On each tile with a hatched strip, cut the strip off along the dashed line."
                .to_owned(),
        ),
        Block::Bullet(
            "Lay each cut edge on the solid line of the tile it joins, so the grey half \
             diamonds make whole ones, then tape the join. Lay the tiles out like this:"
                .to_owned(),
        ),
    ];
    blocks.extend(maps);
    blocks
}

fn cut_section(print: &TestPrint) -> Vec<Block> {
    let mut blocks = vec![Block::Bullet(
        "Cut along the solid black lines, the holes and slots inside the pieces too.".to_owned(),
    )];
    if print
        .items
        .iter()
        .any(|item| !item.lines(LineKind::Perforation).is_empty())
    {
        blocks.push(Block::Bullet(
            "Dotted lines are perforations: prick along them, or cut short slits.".to_owned(),
        ));
    }
    blocks
}

fn fold_section(print: &TestPrint) -> Vec<Block> {
    let mut blocks = vec![Block::Bullet(
        "Score the dashed blue lines first: press along each one with an empty ballpoint pen \
         against a ruler, so the paper folds cleanly."
            .to_owned(),
    )];
    let folds = || print.items.iter().flat_map(|item| &item.folds);
    if folds().next().is_none() {
        return blocks;
    }
    blocks.push(Block::Bullet(
        "Fold in the order of the numbers in the circles on the creases.".to_owned(),
    ));
    blocks.push(Block::Fold {
        step: 1,
        direction: FoldDirection::Mountain,
        text: "White circle: fold away from the printed side, so the printed lines end up \
               outside."
            .to_owned(),
    });
    if folds().any(|fold| fold.direction == FoldDirection::Valley) {
        blocks.push(Block::Fold {
            step: 1,
            direction: FoldDirection::Valley,
            text: "Black circle: fold towards the printed side.".to_owned(),
        });
    }
    if print.items.iter().any(|item| item.part == Part::OuterBox) {
        blocks.push(Block::Bullet(
            "Box pieces are printed on what becomes the outside of the box.".to_owned(),
        ));
    }
    for item in print
        .items
        .iter()
        .filter(|item| item.copy == 1 && !item.folds.is_empty())
    {
        blocks.extend(fold_steps(item));
    }
    blocks
}

/// One piece's fold steps: a line for each step, and for each way and angle within it.
fn fold_steps(item: &TestItem) -> Vec<Block> {
    let label = match item.copies {
        0 | 1 => item.piece_label.clone(),
        _ => format!("{} (each copy)", item.piece_label),
    };
    // Counted by step, then way (away first), then whole degrees.
    let mut groups: BTreeMap<(u32, bool, i64), usize> = BTreeMap::new();
    for fold in &item.folds {
        let towards = fold.direction == FoldDirection::Valley;
        *groups
            .entry((fold.step, towards, fold.fold_deg.round() as i64))
            .or_default() += 1;
    }
    let mut blocks = vec![Block::Subheading(label)];
    for ((step, towards, degrees), count) in groups {
        let creases = match count {
            1 => "the crease".to_owned(),
            _ => format!("the {count} creases"),
        };
        let (direction, way) = match towards {
            false => (FoldDirection::Mountain, "away from the printed side"),
            true => (FoldDirection::Valley, "towards the printed side"),
        };
        blocks.push(Block::Fold {
            step,
            direction,
            text: format!("Fold {creases} marked {step} by {degrees} degrees, {way}."),
        });
    }
    blocks
}

fn slot_section() -> Vec<Block> {
    vec![Block::Bullet(
        "Hold a divider slotted from the bottom over one slotted from the top, and slide \
         them together slot into slot until their edges are level. Do the same at every \
         crossing."
            .to_owned(),
    )]
}

fn fit_section() -> Vec<Block> {
    vec![
        Block::Bullet(
            "Put your product in. It should go in without forcing, with a little room all \
             round."
                .to_owned(),
        ),
        Block::Bullet(
            "Paper is thinner than board, so slots sit loose and folds are softer. Judge the \
             size and the fit, not the strength."
                .to_owned(),
        ),
        Block::Bullet(
            "Too tight or too loose? Change the clearance in your design and print again."
                .to_owned(),
        ),
    ]
}

impl Block {
    /// How much of the page the block takes, in mm.
    fn height(&self, window: Bounds) -> f64 {
        let width = window.length_mm();
        let line = BODY_MM * LEADING;
        match self {
            Block::Title(_) => TITLE_MM * LEADING + AFTER_BLOCK_MM,
            Block::Heading(_) => ABOVE_HEADING_MM + HEADING_MM * LEADING,
            Block::Subheading(_) => ABOVE_SUBHEADING_MM + line,
            Block::Paragraph(text) => {
                wrap((text, BODY_MM, width)).len() as f64 * line + AFTER_BLOCK_MM
            }
            Block::Bullet(text) => {
                wrap((text, BODY_MM, width - BULLET_INDENT_MM)).len() as f64 * line + AFTER_BLOCK_MM
            }
            Block::Fold { text, .. } => {
                let lines = wrap((text, BODY_MM, width - FOLD_INDENT_MM)).len() as f64;
                (lines * line).max(2.0 * FOLD_MARK_RADIUS_MM + 1.0) + AFTER_BLOCK_MM
            }
            Block::Map { columns, rows, .. } => {
                let (_, tile_height) = map_tile((window, *columns));
                line + *rows as f64 * tile_height + 2.0 * AFTER_BLOCK_MM
            }
        }
    }

    /// The block's drawing commands, its top at `y`.
    fn draw(&self, (window, y): (Bounds, f64)) -> String {
        let x = window.min.x;
        let width = window.length_mm();
        match self {
            Block::Title(text) => words((Point::new(x, y + TITLE_MM), TITLE_MM, Font::Bold, text)),
            Block::Heading(text) => words((
                Point::new(x, y + ABOVE_HEADING_MM + HEADING_MM),
                HEADING_MM,
                Font::Bold,
                text,
            )),
            Block::Subheading(text) => words((
                Point::new(x, y + ABOVE_SUBHEADING_MM + BODY_MM),
                BODY_MM,
                Font::Bold,
                text,
            )),
            Block::Paragraph(text) => lines((Point::new(x, y), width, text)),
            Block::Bullet(text) => {
                let mut content = words((
                    Point::new(x + 1.0, y + BODY_MM),
                    BODY_MM,
                    Font::Regular,
                    "-",
                ));
                content.push_str(&lines((
                    Point::new(x + BULLET_INDENT_MM, y),
                    width - BULLET_INDENT_MM,
                    text,
                )));
                content
            }
            Block::Fold {
                step,
                direction,
                text,
            } => {
                let center = Point::new(
                    x + FOLD_MARK_RADIUS_MM + 0.5,
                    y + BODY_MM - FOLD_MARK_RISE_MM,
                );
                let mut content = fold_mark((center, *step, *direction));
                content.push_str(&lines((
                    Point::new(x + FOLD_INDENT_MM, y),
                    width - FOLD_INDENT_MM,
                    text,
                )));
                content
            }
            Block::Map {
                label,
                columns,
                tiles,
                ..
            } => map((window, y, (label, *columns, tiles))),
        }
    }
}

/// One run of text in black, left to right.
fn words((at, size_mm, font, text): (Point, f64, Font, &str)) -> String {
    let mut content = "0 g\n".to_owned();
    content.push_str(&text_at(Text {
        at,
        size_mm,
        turn_deg: 0.0,
        font,
        text,
    }));
    content
}

/// Body text wrapped to `width`, its first line's top at `top`.
fn lines((top, width, text): (Point, f64, &str)) -> String {
    let mut content = String::new();
    for (index, line) in wrap((text, BODY_MM, width)).iter().enumerate() {
        let baseline = top.y + BODY_MM + index as f64 * BODY_MM * LEADING;
        content.push_str(&words((
            Point::new(top.x, baseline),
            BODY_MM,
            Font::Regular,
            line,
        )));
    }
    content
}

/// `text` broken at spaces into lines no wider than `width_mm` at `size_mm`. A word
/// longer than a line gets a line of its own.
fn wrap((text, size_mm, width_mm): (&str, f64, f64)) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if current.is_empty() {
            current = word.to_owned();
            continue;
        }
        let longer = format!("{current} {word}");
        match text_width_mm((&longer, size_mm)) > width_mm {
            true => lines.push(std::mem::replace(&mut current, word.to_owned())),
            false => current = longer,
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

/// A map tile's width and height: up to [`MAP_TILE_MM`] wide, narrower when the columns
/// wouldn't fit, and shaped like the drawing window.
fn map_tile((window, columns): (Bounds, usize)) -> (f64, f64) {
    let width = MAP_TILE_MM.min(window.length_mm() / columns.max(1) as f64);
    (width, width * window.width_mm() / window.length_mm())
}

/// A tiled piece's label, then its printed tiles in their places on a grid `columns`
/// wide, each with its name.
fn map((window, y, (label, columns, tiles)): (Bounds, f64, (&str, usize, &[Tile]))) -> String {
    let x = window.min.x;
    let mut content = words((Point::new(x, y + BODY_MM), BODY_MM, Font::Bold, label));
    let (tile_width, tile_height) = map_tile((window, columns));
    let top = y + BODY_MM * LEADING + AFTER_BLOCK_MM;
    let _ = writeln!(content, "0 G {} w [] 0 d", num(MAP_LINE_MM));
    for tile in tiles {
        let corner = Point::new(
            x + tile.column as f64 * tile_width,
            top + tile.row as f64 * tile_height,
        );
        let _ = writeln!(
            content,
            "{} {} {} {} re S",
            num(corner.x),
            num(corner.y),
            num(tile_width),
            num(tile_height),
        );
        let name = tile.name();
        let width = text_width_mm((&name, MAP_NAME_MM));
        content.push_str(&words((
            Point::new(
                corner.x + (tile_width - width) / 2.0,
                corner.y + tile_height / 2.0 + CAP_HALF_HEIGHT * MAP_NAME_MM,
            ),
            MAP_NAME_MM,
            Font::Bold,
            &name,
        )));
    }
    content
}
