//! Drawing commands shared by the PDF files: the production file (B2) and the
//! print-at-home test print (B4). Everything is drawn in mm with y down the page, from
//! its top-left corner.

use std::fmt::Write;

use crate::export::num;
use crate::geometry::{Path, Point, Segment};

/// PDF points per mm.
pub(crate) const PT_PER_MM: f64 = 72.0 / 25.4;

/// Arcs are split into parts no longer than this.
const MAX_ARC_PART_DEG: f64 = 45.0;

/// Helvetica's advance widths in thousandths of the font size, for the printable ASCII
/// characters (space to tilde), from the font's standard metrics.
const HELVETICA_WIDTHS: [u16; 95] = [
    278, 278, 355, 556, 556, 889, 667, 191, 333, 333, 389, 584, 278, 333, 278,
    278, // space to /
    556, 556, 556, 556, 556, 556, 556, 556, 556, 556, 278, 278, 584, 584, 584, 556, // 0 to ?
    1015, 667, 667, 722, 722, 667, 611, 778, 722, 278, 500, 667, 556, 833, 722, 778, // @ to O
    667, 778, 722, 667, 611, 722, 667, 944, 667, 667, 611, 278, 278, 278, 469, 556, // P to _
    333, 556, 556, 500, 556, 556, 278, 556, 556, 222, 222, 500, 222, 833, 556, 556, // ` to o
    556, 556, 333, 500, 278, 556, 500, 722, 500, 500, 500, 334, 260, 334, 584, // p to ~
];

/// The width used for a character outside printable ASCII: a digit's.
const OTHER_WIDTH: u16 = 556;

/// The commands that open a page's drawing: mm units, y down from the top-left corner of
/// a page `page_height_mm` tall, round caps and joins. Close them with `Q`.
pub(crate) fn page_start(page_height_mm: f64) -> String {
    format!(
        "q\n{k} 0 0 -{k} 0 {} cm\n1 J 1 j\n",
        num(page_height_mm * PT_PER_MM),
        k = num(PT_PER_MM),
    )
}

/// A straight line, stroked.
pub(crate) fn line((from, to): (Point, Point)) -> String {
    format!(
        "{} {} m {} {} l S\n",
        num(from.x),
        num(from.y),
        num(to.x),
        num(to.y)
    )
}

/// The fonts our PDF files use, by the names their pages list them under: Helvetica as
/// `/F1`, Helvetica-Bold as `/F2`. Both are built into every PDF reader, so nothing is
/// embedded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Font {
    Regular,
    Bold,
}

impl Font {
    fn resource(self) -> &'static str {
        match self {
            Font::Regular => "/F1",
            Font::Bold => "/F2",
        }
    }
}

/// One run of text.
pub(crate) struct Text<'a> {
    /// Where the baseline starts, in page mm (y down).
    pub at: Point,
    pub size_mm: f64,
    /// How far the text turns anticlockwise on the page: 0 reads left to right, 90 reads
    /// upwards.
    pub turn_deg: f64,
    pub font: Font,
    pub text: &'a str,
}

/// The text drawing commands. The page is drawn with y down, so the text matrix flips
/// y back to keep the letters the right way up.
pub(crate) fn text_at(text: Text) -> String {
    let size = text.size_mm;
    let (sin, cos) = text.turn_deg.to_radians().sin_cos();
    format!(
        "BT {} 1 Tf {} {} {} {} {} {} Tm ({}) Tj ET\n",
        text.font.resource(),
        num(size * cos),
        num(-size * sin),
        num(-size * sin),
        num(-size * cos),
        num(text.at.x),
        num(text.at.y),
        escape(text.text),
    )
}

/// How wide `text` sets in Helvetica at `size_mm`, in mm. Bold sets a little wider.
pub(crate) fn text_width_mm((text, size_mm): (&str, f64)) -> f64 {
    let ems: f64 = text
        .chars()
        .map(|character| {
            let width = u32::from(character)
                .checked_sub(u32::from(' '))
                .and_then(|index| HELVETICA_WIDTHS.get(index as usize))
                .copied()
                .unwrap_or(OTHER_WIDTH);
            f64::from(width) / 1000.0
        })
        .sum();
    ems * size_mm
}

/// A PDF string's text: backslashes and brackets escaped.
fn escape(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('(', "\\(")
        .replace(')', "\\)")
}

/// The path's construction commands: lines, and arcs as Bézier curves.
pub(crate) fn path_commands(path: &Path) -> String {
    let mut commands = format!("{} {} m\n", num(path.start.x), num(path.start.y));
    let mut current = path.start;
    for segment in &path.segments {
        match *segment {
            Segment::Line { to } => {
                let _ = writeln!(commands, "{} {} l", num(to.x), num(to.y));
                current = to;
            }
            Segment::Arc { center, sweep_deg } => {
                commands.push_str(&arc_curves((current, center, sweep_deg)));
                current = arc_end((current, center, sweep_deg));
            }
        }
    }
    if path.closed {
        commands.push_str("h\n");
    }
    commands
}

/// The arc from `start` round `center` by `sweep_deg`, as cubic Bézier curves of at
/// most [`MAX_ARC_PART_DEG`]. Positive sweeps turn from +x towards +y, as in the engine.
fn arc_curves((start, center, sweep_deg): (Point, Point, f64)) -> String {
    let radius = start.distance_to(center);
    let parts = (sweep_deg.abs() / MAX_ARC_PART_DEG).ceil().max(1.0);
    let part = (sweep_deg / parts).to_radians();
    // Control points sit this far along the tangents, as a share of the radius.
    let reach = 4.0 / 3.0 * (part / 4.0).tan() * radius;
    let first_angle = (start.y - center.y).atan2(start.x - center.x);
    let point_at = |angle: f64| {
        Point::new(
            center.x + radius * angle.cos(),
            center.y + radius * angle.sin(),
        )
    };
    let mut commands = String::new();
    let mut angle = first_angle;
    let mut from = start;
    for _ in 0..parts as usize {
        let next = angle + part;
        let to = point_at(next);
        let control_1 = Point::new(from.x - reach * angle.sin(), from.y + reach * angle.cos());
        let control_2 = Point::new(to.x + reach * next.sin(), to.y - reach * next.cos());
        let _ = writeln!(
            commands,
            "{} {} {} {} {} {} c",
            num(control_1.x),
            num(control_1.y),
            num(control_2.x),
            num(control_2.y),
            num(to.x),
            num(to.y),
        );
        (angle, from) = (next, to);
    }
    commands
}

/// Where an arc from `start` round `center` ends after `sweep_deg`.
fn arc_end((start, center, sweep_deg): (Point, Point, f64)) -> Point {
    let radius = start.distance_to(center);
    let angle = (start.y - center.y).atan2(start.x - center.x) + sweep_deg.to_radians();
    Point::new(
        center.x + radius * angle.cos(),
        center.y + radius * angle.sin(),
    )
}
