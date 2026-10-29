//! Where everything goes in the PNG preview, in px.
//!
//! Top to bottom: a title and the sizes and fit in words; the drawings in rows (the top
//! view first, then every piece flat, insert pieces before box pieces); a legend; a
//! footer. The pieces share one scale, chosen so their rows come out about [`ROW_PX`]
//! wide, at most [`MAX_PX_PER_MM`], and the whole picture at most [`MAX_SIDE_PX`] either
//! way. The top view, the picture of the fit, is drawn larger when that scale leaves it
//! small: about [`TOP_VIEW_PX`] long. Text keeps its size in px whatever the scale.

use super::PngError;
use super::font::{Font, Weight};
use crate::design::{BoxStyle, Design, InsertSpec};
use crate::export::{Sheet, Spacing};
use crate::fit::{FitColor, ProductFit};
use crate::geometry::{Bounds, Point, Polygon};
use crate::insert::{Insert, placed_products};
use crate::material::{Board, Material};
use crate::outer_box::OuterBox;
use crate::packaging::Packaging;

/// The drawings' rows aim at this width.
const ROW_PX: f64 = 1800.0;
/// Rows of drawings come out about this much wider than tall.
const ASPECT: f64 = 1.6;
/// The most px per mm: small designs stop growing here.
const MAX_PX_PER_MM: f64 = 10.0;
/// The picture's longest side at most.
const MAX_SIDE_PX: f64 = 4000.0;
/// The top view's longer side, when the pieces' scale would draw it smaller.
const TOP_VIEW_PX: f64 = 720.0;
/// A top view drawn this many times the pieces' scale or more says so in its label.
const ENLARGED_SHARE: f64 = 1.2;

/// Space round the picture, and between drawings.
const MARGIN_PX: f64 = 56.0;
const GAP_PX: f64 = 56.0;
/// Text sizes, and line spacing as a share of the size.
pub(super) const TITLE_PX: f64 = 44.0;
pub(super) const BODY_PX: f64 = 26.0;
pub(super) const LABEL_PX: f64 = 24.0;
pub(super) const SMALL_PX: f64 = 20.0;
const LEADING: f64 = 1.5;
/// Room above each drawing for its label.
const LABEL_BAND_PX: f64 = 40.0;
/// Legend: each mark this wide, then this gap before its words, then this before the
/// next mark.
pub(super) const LEGEND_MARK_PX: f64 = 30.0;
const LEGEND_TEXT_GAP_PX: f64 = 10.0;
const LEGEND_ITEM_GAP_PX: f64 = 36.0;

const TITLE: &str = "Packaging preview";
const FOOTER: &str = "Made with Paccradle. A preview, not a production file: box pieces \
                      are shown from the outside.";

/// The PNG preview laid out. [`Preview::to_png`] draws it.
#[derive(Debug, Clone, PartialEq)]
pub struct Preview {
    pub width_px: u32,
    pub height_px: u32,
    /// Px per mm in the pieces' drawings.
    pub scale: f64,
    /// The title, the sizes and the fit, one line each, with where each baseline starts.
    pub header: Vec<Line>,
    pub blocks: Vec<Block>,
    pub legend: Vec<LegendItem>,
    pub footer: Line,
    /// What the drawings show.
    pub(super) parts: Parts,
}

/// A line of text and where its baseline starts, in px.
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    pub text: String,
    pub at: Point,
    pub size_px: f64,
    pub bold: bool,
}

/// One drawing, with its label above it.
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub kind: BlockKind,
    pub label: Line,
    /// The drawing's own top-left corner, in its own mm (the insert's frame for the top
    /// view, the production sheet's for a piece).
    pub origin_mm: Point,
    pub size_mm: (f64, f64),
    /// Where `origin_mm` lands in the picture.
    pub at_px: Point,
    /// Px per mm in this drawing.
    pub scale: f64,
}

/// What a drawing shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
    /// The insert from above, in the box's inside, products coloured by fit.
    TopView,
    /// One piece flat: index into the production sheet's pieces
    /// ([`Sheet::of`](crate::export::Sheet::of)).
    Piece(usize),
}

/// One entry of the legend: its mark, starting at `at` on the baseline, then its words.
#[derive(Debug, Clone, PartialEq)]
pub struct LegendItem {
    pub mark: LegendMark,
    pub text: &'static str,
    pub at: Point,
}

/// The legend's marks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegendMark {
    Fit(FitColor),
    Cut,
    Crease,
    FoldStep,
}

const LEGEND: [(LegendMark, &str); 6] = [
    (LegendMark::Fit(FitColor::Green), "fits with room"),
    (LegendMark::Fit(FitColor::Yellow), "tight"),
    (LegendMark::Fit(FitColor::Red), "doesn't fit"),
    (LegendMark::Cut, "cut"),
    (LegendMark::Crease, "fold"),
    (
        LegendMark::FoldStep,
        "fold order: fold away from the side shown",
    ),
];

/// What the drawings show, from the preview's geometry.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Parts {
    /// Every piece once, box pieces mirrored print side up, as the production files.
    pub sheet: Sheet,
    /// The insert, for the top view.
    pub insert: Insert,
    /// The insert's board thickness: how wide an upright strip looks from above.
    pub thickness_mm: f64,
    /// The box's inside, in the insert's frame.
    pub inside: Option<Bounds>,
    /// Each product copy's outline in the insert's frame, and its fit.
    pub products: Vec<(Polygon, FitColor)>,
}

impl Preview {
    /// Lays out the preview of `packaging`, generated from `design`.
    pub fn of((packaging, design): (&Packaging, &Design)) -> Result<Preview, PngError> {
        let fits = packaging.fits(design)?;
        let preview = packaging.rounded_for_preview();
        let products = placed_products((design, &packaging.insert))?
            .into_iter()
            .map(|placed| {
                let color = fits
                    .iter()
                    .find(|fit| fit.placement == placed.placement)
                    .map_or(FitColor::Red, |fit| fit.color);
                (placed.outline, color)
            })
            .collect();
        let parts = Parts {
            sheet: Sheet::of((&preview, Spacing::CUT_FILE)),
            inside: packaging
                .outer_box
                .as_ref()
                .map(|outer_box| inside((&preview.insert, outer_box))),
            insert: preview.insert,
            thickness_mm: design.material.thickness_mm,
            products,
        };
        let header = header_lines((design, packaging, &fits));
        let regular = Font::new(Weight::Regular)?;
        let bold = Font::new(Weight::Bold)?;
        Ok(Arrange {
            parts,
            header,
            regular: &regular,
            bold: &bold,
        }
        .preview())
    }

    /// Where a point of `block`'s drawing lands in the picture.
    pub fn to_px(&self, (block, point): (&Block, Point)) -> Point {
        Point::new(
            block.at_px.x + (point.x - block.origin_mm.x) * block.scale,
            block.at_px.y + (point.y - block.origin_mm.y) * block.scale,
        )
    }
}

/// A drawing before it's placed: what it shows, its label, and its own top-left corner
/// and size in mm.
struct Drawing {
    kind: BlockKind,
    label: String,
    origin_mm: Point,
    size_mm: (f64, f64),
}

/// Everything to lay out, and the fonts to measure text with.
struct Arrange<'a> {
    parts: Parts,
    header: Vec<String>,
    regular: &'a Font,
    bold: &'a Font,
}

impl Arrange<'_> {
    /// The layout at the scale the drawings' sizes suggest, shrunk until it fits
    /// [`MAX_SIDE_PX`].
    fn preview(self) -> Preview {
        let drawings = self.drawings();
        let sizes: Vec<(f64, f64)> = drawings.iter().map(|drawing| drawing.size_mm).collect();
        let mut scale = first_scale(&sizes);
        let mut preview = self.at_scale((&drawings, scale));
        for _ in 0..8 {
            let longest = f64::from(preview.width_px.max(preview.height_px));
            if longest <= MAX_SIDE_PX {
                break;
            }
            scale *= MAX_SIDE_PX / longest * 0.98;
            preview = self.at_scale((&drawings, scale));
        }
        preview
    }

    /// Every drawing, in order: the top view, then the sheet's pieces.
    fn drawings(&self) -> Vec<Drawing> {
        let view = top_view_bounds(&self.parts);
        let label = match self.parts.inside {
            Some(_) => "Top view in the box",
            None => "Top view",
        };
        let top_view = Drawing {
            kind: BlockKind::TopView,
            label: label.to_owned(),
            origin_mm: view.min,
            size_mm: (view.length_mm(), view.width_mm()),
        };
        let pieces = self
            .parts
            .sheet
            .pieces
            .iter()
            .enumerate()
            .map(|(index, piece)| {
                let bounds = piece.bounds;
                let cut = match piece.quantity {
                    0 | 1 => String::new(),
                    quantity => format!(" (cut {quantity})"),
                };
                Drawing {
                    kind: BlockKind::Piece(index),
                    label: format!(
                        "{}{cut} - {} x {} mm",
                        piece.label(),
                        mm(bounds.length_mm()),
                        mm(bounds.width_mm())
                    ),
                    origin_mm: bounds.min,
                    size_mm: (bounds.length_mm(), bounds.width_mm()),
                }
            });
        std::iter::once(top_view).chain(pieces).collect()
    }

    /// The whole layout with the drawings at `scale` px per mm.
    fn at_scale(&self, (drawings, scale): (&[Drawing], f64)) -> Preview {
        // Header: title, then a line each.
        let mut y = MARGIN_PX;
        let mut header = Vec::new();
        y += TITLE_PX;
        header.push(Line {
            text: TITLE.to_owned(),
            at: Point::new(MARGIN_PX, y),
            size_px: TITLE_PX,
            bold: true,
        });
        y += TITLE_PX * (LEADING - 1.0);
        for text in &self.header {
            y += BODY_PX * LEADING;
            header.push(Line {
                text: text.clone(),
                at: Point::new(MARGIN_PX, y),
                size_px: BODY_PX,
                bold: false,
            });
        }
        let header_width = header
            .iter()
            .map(|line| self.width((line, line.bold)))
            .fold(0.0, f64::max);

        // Each drawing at its scale and with its label.
        let placed: Vec<(f64, String)> = drawings
            .iter()
            .map(|drawing| {
                let own = drawing_scale((drawing, scale));
                let label = match own >= ENLARGED_SHARE * scale {
                    true => format!("{} (enlarged)", drawing.label),
                    false => drawing.label.clone(),
                };
                (own, label)
            })
            .collect();
        // Drawings in rows, each box its drawing or its label, whichever is wider.
        let boxes: Vec<(f64, f64)> = drawings
            .iter()
            .zip(&placed)
            .map(|(drawing, (own, label))| {
                let (length, width) = drawing.size_mm;
                let label_width = self.bold.width_px((label, LABEL_PX));
                ((length * own).max(label_width), LABEL_BAND_PX + width * own)
            })
            .collect();
        let widest = boxes.iter().map(|(width, _)| *width).fold(0.0, f64::max);
        let rows_top = y + GAP_PX;
        let packed = pack((&boxes, ROW_PX.max(widest)));
        let blocks = drawings
            .iter()
            .zip(&placed)
            .zip(&packed.corners)
            .map(|((drawing, (own, label)), corner)| {
                let corner = Point::new(MARGIN_PX + corner.x, rows_top + corner.y);
                Block {
                    kind: drawing.kind,
                    label: Line {
                        text: label.clone(),
                        at: Point::new(corner.x, corner.y + LABEL_PX),
                        size_px: LABEL_PX,
                        bold: true,
                    },
                    origin_mm: drawing.origin_mm,
                    size_mm: drawing.size_mm,
                    at_px: Point::new(corner.x, corner.y + LABEL_BAND_PX),
                    scale: *own,
                }
            })
            .collect();

        // The legend and footer under the drawings, as wide as the rest allows.
        let content_width = packed.width.max(header_width).max(ROW_PX);
        let legend_top = rows_top + packed.height + GAP_PX;
        let (legend, legend_bottom) = self.legend((legend_top, content_width));
        let footer = Line {
            text: FOOTER.to_owned(),
            at: Point::new(MARGIN_PX, legend_bottom + SMALL_PX * LEADING),
            size_px: SMALL_PX,
            bold: false,
        };
        let width = content_width.max(self.width((&footer, false))) + 2.0 * MARGIN_PX;
        let height = footer.at.y + SMALL_PX * (LEADING - 1.0) + MARGIN_PX;
        Preview {
            width_px: width.ceil() as u32,
            height_px: height.ceil() as u32,
            scale,
            header,
            blocks,
            legend,
            footer,
            parts: self.parts.clone(),
        }
    }

    /// The legend's items left to right from `top`, wrapping at `width`; and where it
    /// ends.
    fn legend(&self, (top, width): (f64, f64)) -> (Vec<LegendItem>, f64) {
        let mut items = Vec::with_capacity(LEGEND.len());
        let mut baseline = top + BODY_PX;
        let mut x = 0.0;
        for (mark, text) in LEGEND {
            let item_width =
                LEGEND_MARK_PX + LEGEND_TEXT_GAP_PX + self.regular.width_px((text, BODY_PX));
            if x > 0.0 && x + item_width > width {
                baseline += BODY_PX * LEADING;
                x = 0.0;
            }
            items.push(LegendItem {
                mark,
                text,
                at: Point::new(MARGIN_PX + x, baseline),
            });
            x += item_width + LEGEND_ITEM_GAP_PX;
        }
        (items, baseline)
    }

    fn width(&self, (line, bold): (&Line, bool)) -> f64 {
        let font = match bold {
            true => self.bold,
            false => self.regular,
        };
        font.width_px((&line.text, line.size_px))
    }
}

/// A drawing's px per mm: the pieces' `scale`, or for the top view enough to draw it
/// about [`TOP_VIEW_PX`] long, if that's more (at most [`MAX_PX_PER_MM`]).
fn drawing_scale((drawing, scale): (&Drawing, f64)) -> f64 {
    let (length, width) = drawing.size_mm;
    let longer = length.max(width);
    match (drawing.kind, longer > 0.0) {
        (BlockKind::TopView, true) => (TOP_VIEW_PX / longer).min(MAX_PX_PER_MM).max(scale),
        _ => scale,
    }
}

/// Where the legend's words start after its mark.
pub(super) const fn legend_text_offset() -> f64 {
    LEGEND_MARK_PX + LEGEND_TEXT_GAP_PX
}

/// The scale that makes the drawings' rows about [`ROW_PX`] wide, when they're packed
/// about [`ASPECT`] times wider than tall; at most [`MAX_PX_PER_MM`].
fn first_scale(sizes: &[(f64, f64)]) -> f64 {
    let widest = sizes.iter().map(|(length, _)| *length).fold(0.0, f64::max);
    let area: f64 = sizes.iter().map(|(length, width)| length * width).sum();
    let row_mm = widest.max((area * ASPECT).sqrt());
    match row_mm > 0.0 {
        true => (ROW_PX / row_mm).min(MAX_PX_PER_MM),
        false => MAX_PX_PER_MM,
    }
}

/// Boxes placed in rows, left to right, a row ending before `row_px`.
struct Packed {
    corners: Vec<Point>,
    width: f64,
    height: f64,
}

fn pack((boxes, row_px): (&[(f64, f64)], f64)) -> Packed {
    let mut corners = Vec::with_capacity(boxes.len());
    let (mut x, mut y, mut row_height, mut width) = (0.0, 0.0, 0.0_f64, 0.0_f64);
    for &(box_width, box_height) in boxes {
        if x > 0.0 && x + box_width > row_px {
            y += row_height + GAP_PX;
            (x, row_height) = (0.0, 0.0);
        }
        corners.push(Point::new(x, y));
        width = width.max(x + box_width);
        row_height = row_height.max(box_height);
        x += box_width + GAP_PX;
    }
    Packed {
        corners,
        width,
        height: y + row_height,
    }
}

/// The box's inside in the insert's frame: centred on the insert.
fn inside((insert, outer_box): (&Insert, &OuterBox)) -> Bounds {
    let size = outer_box.inside_mm;
    let corner = Point::new(
        (insert.length_mm - size.length_mm) / 2.0,
        (insert.width_mm - size.width_mm) / 2.0,
    );
    Bounds {
        min: corner,
        max: Point::new(corner.x + size.length_mm, corner.y + size.width_mm),
    }
}

/// What the top view shows: the box's inside (or the insert), and any product hanging
/// out of it.
fn top_view_bounds(parts: &Parts) -> Bounds {
    let insert = Bounds::from_size((parts.insert.length_mm, parts.insert.width_mm));
    let view = parts.inside.unwrap_or(insert);
    parts
        .products
        .iter()
        .filter_map(|(outline, _)| outline.bounds())
        .fold(view, Bounds::union)
}

/// The sizes and the fit, in words.
fn header_lines((design, packaging, fits): (&Design, &Packaging, &[ProductFit])) -> Vec<String> {
    let insert = &packaging.insert;
    let mut lines = vec![format!(
        "Insert: {}, {} - {} x {} x {} mm",
        style_name(&design.insert),
        material_name(&design.material),
        mm(insert.length_mm),
        mm(insert.width_mm),
        mm(insert.height_mm),
    )];
    if let (Some(outer_box), Some(spec)) = (&packaging.outer_box, &design.outer_box) {
        let (inside, outside) = (outer_box.inside_mm, outer_box.outside_mm);
        lines.push(format!(
            "Box: {}, {} - inside {} x {} x {} mm, outside {} x {} x {} mm",
            box_name(outer_box.style),
            material_name(&spec.material),
            mm(inside.length_mm),
            mm(inside.width_mm),
            mm(inside.height_mm),
            mm(outside.length_mm),
            mm(outside.width_mm),
            mm(outside.height_mm),
        ));
    }
    lines.push(fit_sentence((design, fits)));
    lines
}

/// "Fit: all 6 products have at least the 1.5 mm room we suggest for E-flute.", or how
/// many are tight and don't fit.
fn fit_sentence((design, fits): (&Design, &[ProductFit])) -> String {
    let board = board_name(design.material.board);
    let target = mm(Material::starting_values(design.material.board).clearance_mm);
    let count = |color: FitColor| fits.iter().filter(|fit| fit.color == color).count();
    let (green, yellow, red) = (
        count(FitColor::Green),
        count(FitColor::Yellow),
        count(FitColor::Red),
    );
    if fits.is_empty() {
        return "Fit: no products placed.".to_owned();
    }
    if yellow == 0 && red == 0 {
        let who = match green {
            1 => "the product has".to_owned(),
            2 => "both products have".to_owned(),
            _ => format!("all {green} products have"),
        };
        return format!("Fit: {who} at least the {target} mm room we suggest for {board}.");
    }
    let parts: Vec<String> = [
        (green, "with room"),
        (yellow, "tight"),
        (red, "not fitting"),
    ]
    .into_iter()
    .filter(|(count, _)| *count > 0)
    .map(|(count, what)| format!("{count} {what}"))
    .collect();
    format!(
        "Fit: {} (we suggest {target} mm room for {board}).",
        parts.join(", ")
    )
}

fn style_name(spec: &InsertSpec) -> &'static str {
    match spec {
        InsertSpec::PartitionGrid(_) => "partition grid",
        InsertSpec::CutoutCard(_) => "cutout card",
        InsertSpec::CradlePlatform(_) => "cradle platform",
    }
}

fn box_name(style: BoxStyle) -> &'static str {
    match style {
        BoxStyle::Mailer => "mailer box",
        BoxStyle::Shipping => "shipping box",
    }
}

fn board_name(board: Board) -> &'static str {
    match board {
        Board::Paperboard => "paperboard",
        Board::Chipboard => "chipboard",
        Board::EFlute => "E-flute",
        Board::BFlute => "B-flute",
        Board::CFlute => "C-flute",
    }
}

/// "chipboard 1.5 mm".
fn material_name(material: &Material) -> String {
    format!(
        "{} {} mm",
        board_name(material.board),
        mm(material.thickness_mm)
    )
}

/// A length to a tenth of a mm, without a trailing ".0".
fn mm(value: f64) -> String {
    let text = format!("{value:.1}");
    match text.strip_suffix(".0") {
        Some(whole) => whole.to_owned(),
        None => text,
    }
}
