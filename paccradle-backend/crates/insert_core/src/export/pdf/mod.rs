//! PDF production file (M1-B2, CLAUDE.md §6.6): for printers and die makers, and for
//! checking a design on paper.
//!
//! - One page at true size: 1 mm on the drawing prints as 1 mm at 100 %.
//! - Layers (optional content groups, which Acrobat and Illustrator show as layers):
//!   CUT (red, solid), CREASE (blue, dashed), PERFORATION (green, dotted; only when a
//!   piece has perforations), DIMENSIONS (each piece's length and width) and LABELS (each
//!   piece's name and how many to cut, and a legend). Text and dimensions never sit on
//!   CUT or CREASE, so a die maker can switch them off.
//! - Written by hand: lines, colours, layers and text in Helvetica, one of the fonts every
//!   PDF reader has built in, are all it needs. Nothing is compressed, so the file is
//!   plain ASCII.
//! - Arcs become cubic Bézier curves of at most 45°, within 0.00001 × the radius.

pub(crate) mod draw;
pub(crate) mod file;

use std::fmt::Write;

use self::draw::{Font, PT_PER_MM, Text, line, path_commands, text_at, text_width_mm};
use self::file::write_file;
use super::num;
use super::sheet::{LineKind, Sheet, SheetPiece};
use crate::geometry::{Bounds, Point};

/// The page is at least this wide, so the legend fits beside a small sheet.
const MIN_PAGE_MM: f64 = 150.0;

/// Cut, crease and perforation lines: thin, but visible on screen and on paper.
const LINE_MM: f64 = 0.1;
const CREASE_DASH_MM: [f64; 2] = [3.0, 1.5];
const PERFORATION_DASH_MM: [f64; 2] = [1.0, 1.0];

/// Dimensions: grey lines this wide, this far from the piece, with arrowheads.
const DIMENSION_GREY: f64 = 0.35;
const DIMENSION_LINE_MM: f64 = 0.15;
const DIMENSION_OFFSET_MM: f64 = 8.0;
/// Extension lines start this far from the piece and run this far past the dimension.
const EXTENSION_GAP_MM: f64 = 1.5;
const EXTENSION_PAST_MM: f64 = 1.5;
const ARROW_LENGTH_MM: f64 = 2.0;
const ARROW_HALF_WIDTH_MM: f64 = 0.6;
/// Text sits this far off the line it labels.
const TEXT_CLEARANCE_MM: f64 = 1.0;

/// Text heights (the font size, in mm).
const DIMENSION_TEXT_MM: f64 = 2.5;
const LABEL_TEXT_MM: f64 = 3.5;
const LEGEND_TEXT_MM: f64 = 2.5;
/// A piece's label baseline sits this far above it, over the length dimension.
const LABEL_ABOVE_MM: f64 = 15.0;
/// The legend's lines, and their spacing, from the bottom of the page.
const LEGEND: [&str; 2] = [
    "Paccradle dieline - units: mm - print at 100 % (actual size)",
    "CUT: red, solid - CREASE: blue, dashed - box pieces are mirrored, print side up",
];
const LEGEND_LINE_MM: f64 = 4.0;
const LEGEND_BOTTOM_MM: f64 = 8.0;

/// The page's object numbers, after the catalog (1). Layers follow from
/// [`FIRST_LAYER_OBJECT`].
const PAGES_OBJECT: usize = 2;
const PAGE_OBJECT: usize = 3;
const FONT_OBJECT: usize = 4;
const CONTENT_OBJECT: usize = 5;
const INFO_OBJECT: usize = 6;
const FIRST_LAYER_OBJECT: usize = 7;

/// One layer of the page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Layer {
    Lines(LineKind),
    Dimensions,
    Labels,
}

impl Layer {
    fn name(self) -> &'static str {
        match self {
            Layer::Lines(kind) => kind.layer_name(),
            Layer::Dimensions => "DIMENSIONS",
            Layer::Labels => "LABELS",
        }
    }
}

impl Sheet {
    /// The sheet as a one-page PDF file.
    pub fn to_pdf(&self) -> Vec<u8> {
        let layers = self.layers();
        let page = (self.length_mm.max(MIN_PAGE_MM), self.width_mm);
        let content = self.content((&layers, page));
        document((&layers, page, &content)).into_bytes()
    }

    /// The layers this sheet needs, in order: PERFORATION only when a piece has some.
    fn layers(&self) -> Vec<Layer> {
        let perforated = self
            .pieces
            .iter()
            .any(|piece| !piece.perforations.is_empty());
        LineKind::ALL
            .into_iter()
            .filter(|&kind| kind != LineKind::Perforation || perforated)
            .map(Layer::Lines)
            .chain([Layer::Dimensions, Layer::Labels])
            .collect()
    }

    /// The page's drawing commands. Everything is drawn in mm with y down the page, from
    /// its top-left corner, like the sheet.
    fn content(&self, (layers, (_, page_width_mm)): (&[Layer], (f64, f64))) -> String {
        let mut content = draw::page_start(page_width_mm);
        for (index, &layer) in layers.iter().enumerate() {
            let _ = writeln!(content, "/OC /L{index} BDC\nq");
            content.push_str(&match layer {
                Layer::Lines(kind) => self.lines(kind),
                Layer::Dimensions => self.dimensions(),
                Layer::Labels => self.labels(page_width_mm),
            });
            content.push_str("Q\nEMC\n");
        }
        content.push_str("Q\n");
        content
    }

    /// Every piece's lines of one kind, in the kind's colour and dash.
    fn lines(&self, kind: LineKind) -> String {
        let (color, dash) = match kind {
            LineKind::Cut => ("1 0 0", None),
            LineKind::Crease => ("0 0 1", Some(CREASE_DASH_MM)),
            LineKind::Perforation => ("0 0.627 0", Some(PERFORATION_DASH_MM)),
        };
        let dash = dash.map_or("[] 0 d".to_owned(), |[on, off]| {
            format!("[{} {}] 0 d", num(on), num(off))
        });
        let mut content = format!("{color} RG {} w {dash}\n", num(LINE_MM));
        for path in self.pieces.iter().flat_map(|piece| piece.lines(kind)) {
            content.push_str(&path_commands(path));
            content.push_str("S\n");
        }
        content
    }

    /// Each piece's length above it and width to its left, in mm to one decimal.
    fn dimensions(&self) -> String {
        let mut content = format!(
            "{grey} RG {grey} g {} w [] 0 d\n",
            num(DIMENSION_LINE_MM),
            grey = num(DIMENSION_GREY),
        );
        for piece in &self.pieces {
            content.push_str(&length_dimension(piece.bounds));
            content.push_str(&width_dimension(piece.bounds));
        }
        content
    }

    /// Each piece's title above it, and the legend at the foot of the page.
    fn labels(&self, page_width_mm: f64) -> String {
        let mut content = "0 g\n".to_owned();
        for piece in &self.pieces {
            content.push_str(&label(piece));
        }
        for (line, text) in LEGEND.iter().rev().enumerate() {
            let baseline = page_width_mm - LEGEND_BOTTOM_MM - LEGEND_LINE_MM * line as f64;
            content.push_str(&text_at(Text {
                at: Point::new(self.spacing.margin_mm, baseline),
                size_mm: LEGEND_TEXT_MM,
                turn_deg: 0.0,
                font: Font::Regular,
                text,
            }));
        }
        content
    }
}

/// The piece's title, left-aligned above its dimensions.
fn label(piece: &SheetPiece) -> String {
    text_at(Text {
        at: Point::new(piece.bounds.min.x, piece.bounds.min.y - LABEL_ABOVE_MM),
        size_mm: LABEL_TEXT_MM,
        turn_deg: 0.0,
        font: Font::Regular,
        text: &piece.title(),
    })
}

/// The piece's length, on a line above it.
fn length_dimension(bounds: Bounds) -> String {
    let y = bounds.min.y - DIMENSION_OFFSET_MM;
    let (from, to) = (Point::new(bounds.min.x, y), Point::new(bounds.max.x, y));
    let mut content = String::new();
    for x in [bounds.min.x, bounds.max.x] {
        content.push_str(&line((
            Point::new(x, bounds.min.y - EXTENSION_GAP_MM),
            Point::new(x, y - EXTENSION_PAST_MM),
        )));
    }
    content.push_str(&dimension_line((from, to)));
    let value = dimension_value(bounds.length_mm());
    let middle = (bounds.min.x + bounds.max.x) / 2.0;
    content.push_str(&text_at(Text {
        at: Point::new(
            middle - text_width_mm((&value, DIMENSION_TEXT_MM)) / 2.0,
            y - TEXT_CLEARANCE_MM,
        ),
        size_mm: DIMENSION_TEXT_MM,
        turn_deg: 0.0,
        font: Font::Regular,
        text: &value,
    }));
    content
}

/// The piece's width, on a line to its left, its number reading upwards.
fn width_dimension(bounds: Bounds) -> String {
    let x = bounds.min.x - DIMENSION_OFFSET_MM;
    let (from, to) = (Point::new(x, bounds.min.y), Point::new(x, bounds.max.y));
    let mut content = String::new();
    for y in [bounds.min.y, bounds.max.y] {
        content.push_str(&line((
            Point::new(bounds.min.x - EXTENSION_GAP_MM, y),
            Point::new(x - EXTENSION_PAST_MM, y),
        )));
    }
    content.push_str(&dimension_line((from, to)));
    let value = dimension_value(bounds.width_mm());
    let middle = (bounds.min.y + bounds.max.y) / 2.0;
    content.push_str(&text_at(Text {
        at: Point::new(
            x - TEXT_CLEARANCE_MM,
            middle + text_width_mm((&value, DIMENSION_TEXT_MM)) / 2.0,
        ),
        size_mm: DIMENSION_TEXT_MM,
        turn_deg: 90.0,
        font: Font::Regular,
        text: &value,
    }));
    content
}

/// "142.0": a dimension to a tenth of a mm.
fn dimension_value(mm: f64) -> String {
    format!("{mm:.1}")
}

/// A dimension line with an arrowhead at each end.
fn dimension_line((from, to): (Point, Point)) -> String {
    let mut content = line((from, to));
    content.push_str(&arrowhead((from, to)));
    content.push_str(&arrowhead((to, from)));
    content
}

/// A filled arrowhead with its tip at `tip`, pointing away from `toward`.
fn arrowhead((tip, toward): (Point, Point)) -> String {
    let length = tip.distance_to(toward);
    if length == 0.0 {
        return String::new();
    }
    let (ux, uy) = ((toward.x - tip.x) / length, (toward.y - tip.y) / length);
    let base = Point::new(tip.x + ux * ARROW_LENGTH_MM, tip.y + uy * ARROW_LENGTH_MM);
    let (nx, ny) = (-uy * ARROW_HALF_WIDTH_MM, ux * ARROW_HALF_WIDTH_MM);
    format!(
        "{} {} m {} {} l {} {} l h f\n",
        num(tip.x),
        num(tip.y),
        num(base.x + nx),
        num(base.y + ny),
        num(base.x - nx),
        num(base.y - ny),
    )
}

/// The whole file: catalog with the layers, one page, Helvetica, the content, the
/// document info and one layer object each.
fn document((layers, (length_mm, width_mm), content): (&[Layer], (f64, f64), &str)) -> String {
    let layer_refs: Vec<String> = (0..layers.len())
        .map(|index| format!("{} 0 R", FIRST_LAYER_OBJECT + index))
        .collect();
    let layer_refs = layer_refs.join(" ");
    let properties: Vec<String> = (0..layers.len())
        .map(|index| format!("/L{index} {} 0 R", FIRST_LAYER_OBJECT + index))
        .collect();
    let mut objects = vec![
        format!(
            "<< /Type /Catalog /Pages {PAGES_OBJECT} 0 R /OCProperties << /OCGs [{layer_refs}] \
             /D << /Name (Layers) /Order [{layer_refs}] /ON [{layer_refs}] >> >> >>"
        ),
        format!("<< /Type /Pages /Kids [{PAGE_OBJECT} 0 R] /Count 1 >>"),
        format!(
            "<< /Type /Page /Parent {PAGES_OBJECT} 0 R /MediaBox [0 0 {} {}] \
             /Resources << /Font << /F1 {FONT_OBJECT} 0 R >> /Properties << {} >> >> \
             /Contents {CONTENT_OBJECT} 0 R >>",
            num(length_mm * PT_PER_MM),
            num(width_mm * PT_PER_MM),
            properties.join(" "),
        ),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>"
            .to_owned(),
        format!(
            "<< /Length {} >>\nstream\n{content}endstream",
            content.len()
        ),
        "<< /Title (Paccradle dieline) /Producer (Paccradle insert_core) >>".to_owned(),
    ];
    objects.extend(
        layers
            .iter()
            .map(|layer| format!("<< /Type /OCG /Name ({}) >>", layer.name())),
    );
    write_file((&objects, INFO_OBJECT))
}
