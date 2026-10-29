//! Drawing the test print's pages, and the file round them.
//!
//! Each page: its lines cut down to what it shows, fold numbers on the creases, labels
//! or seam marks, the footer with the page number and scale bar, then the watermark over
//! everything. There are no layers: cut and crease lines, marks and watermark are one
//! drawing.

use std::fmt::Write;

use super::{
    MARGIN_MM, OVERLAP_MM, Paper, Polyline, ScaleBar, TestPage, TestPrint, Tile, View, inside,
    instructions, tile_name,
};
use crate::export::marks::{KeepOut, mark_spots};
use crate::export::num;
use crate::export::pdf::draw::{
    self, Font, PT_PER_MM, Text, line, path_commands, text_at, text_width_mm,
};
use crate::export::pdf::file::write_file;
use crate::export::{LineKind, jitter};
use crate::geometry::{Bounds, Path, Point};
use crate::piece::FoldDirection;

/// Fold marks: a circle this big round the step number.
pub(super) const FOLD_MARK_RADIUS_MM: f64 = 2.4;
const FOLD_NUMBER_MM: f64 = 3.0;
/// Helvetica's digits stand about this share of the font size above the baseline, halved:
/// how far below the middle a centred number's baseline goes.
const DIGIT_HALF_HEIGHT: f64 = 0.36;

/// A whole piece's label: its baseline this far above the piece.
const LABEL_TEXT_MM: f64 = 3.0;
const LABEL_ABOVE_MM: f64 = 2.0;

/// The trim strip's hatching: grey lines at 45°, this far apart.
const HATCH_MM: f64 = 2.5;
const HATCH_GREY: f64 = 0.7;
const HATCH_LINE_MM: f64 = 0.15;

/// Half the overlap: from the window's edge to the seam.
const HALF_OVERLAP_MM: f64 = OVERLAP_MM / 2.0;

/// Seams: grey lines, and grey diamonds that join up across the seam.
const SEAM_GREY: f64 = 0.3;
const SEAM_LINE_MM: f64 = 0.25;
const SEAM_DASH_MM: [f64; 2] = [1.5, 1.0];
const DIAMOND_GREY: f64 = 0.55;
/// Half a diamond's size across the seam, and along it.
const DIAMOND_ACROSS_MM: f64 = 2.5;
const DIAMOND_ALONG_MM: f64 = 4.0;
/// Diamonds per seam, spread evenly. An even number leaves the middle free for the note.
const DIAMONDS: u32 = 4;
const NOTE_MM: f64 = 2.2;
const NOTE_GREY: f64 = 0.3;

/// Footer: a grey rule under the drawing, then the page line and scale bar on one
/// baseline.
const RULE_GREY: f64 = 0.6;
const RULE_BELOW_MM: f64 = 2.5;
const FOOTER_TEXT_MM: f64 = 2.6;
const FOOTER_BASELINE_MM: f64 = 2.5;
const SCALE_BAR_MM: f64 = 2.0;
/// Space between the scale bar's label and the bar.
const SCALE_LABEL_GAP_MM: f64 = 2.0;

/// The watermark: rows of bold text across the whole page, turned, see-through, drawn
/// over the lines. Each page shifts its rows by its own amounts, so the pattern doesn't
/// repeat from page to page.
const WATERMARK: &str = "PACCRADLE TEST PRINT - NOT FOR PRODUCTION";
const WATERMARK_GAP: &str = "      ";
const WATERMARK_MM: f64 = 5.0;
const WATERMARK_ROW_MM: f64 = 22.0;
const WATERMARK_TURN_DEG: f64 = 30.0;
const WATERMARK_GREY: f64 = 0.25;
const WATERMARK_OPACITY: f64 = 0.2;

/// What the instruction pages' footers say.
const GUIDE_FOOTER: &str = "how to build it";

/// The file's object numbers. Each page then takes two: the page and its content.
const PAGES_OBJECT: usize = 2;
const FONT_OBJECT: usize = 3;
const BOLD_FONT_OBJECT: usize = 4;
const WATERMARK_STATE_OBJECT: usize = 5;
const INFO_OBJECT: usize = 6;
const FIRST_PAGE_OBJECT: usize = 7;

/// A page's footer.
struct Footer<'a> {
    /// The page's number, from 1.
    number: usize,
    total: usize,
    /// What the page shows.
    what: &'a str,
}

impl TestPrint {
    /// The test print as a PDF file: the instructions, then one page per [`TestPage`].
    pub fn to_pdf(&self) -> Vec<u8> {
        let guide = instructions::pages(self)
            .into_iter()
            .map(|body| (body, GUIDE_FOOTER.to_owned()));
        let drawings = self
            .pages
            .iter()
            .map(|page| (self.drawing(page), self.what(page)));
        let bodies: Vec<(String, String)> = guide.chain(drawings).collect();
        let total = bodies.len();
        let contents: Vec<String> = bodies
            .iter()
            .enumerate()
            .map(|(index, (body, what))| {
                self.page((
                    body,
                    Footer {
                        number: index + 1,
                        total,
                        what,
                    },
                ))
            })
            .collect();
        document((self.paper.size_mm(), &contents)).into_bytes()
    }

    /// A whole page's drawing commands round its `body`.
    fn page(&self, (body, footer): (&str, Footer)) -> String {
        let (_, height) = self.paper.size_mm();
        let mut content = draw::page_start(height);
        content.push_str(body);
        content.push_str(&self.footer(&footer));
        content.push_str(&watermark((self.paper, footer.number)));
        content.push_str("Q\n");
        content
    }

    /// What a drawing page shows, for its footer.
    fn what(&self, page: &TestPage) -> String {
        let item = page
            .views
            .first()
            .and_then(|view| self.items.get(view.item));
        match (page.tile, item) {
            (Some(tile), Some(item)) => format!(
                "{} - tile {} of {} x {}",
                item.label(),
                tile.name(),
                counted((tile.rows, "row")),
                counted((tile.columns, "column")),
            ),
            _ => "whole pieces".to_owned(),
        }
    }

    /// A drawing page's body: trim strips under the lines, the lines, fold marks, then
    /// labels (whole pieces) or seams (a tile).
    fn drawing(&self, page: &TestPage) -> String {
        let window = self.paper.window();
        let mut content = String::new();
        if let Some(tile) = page.tile {
            content.push_str(&trim_strips((window, tile)));
        }
        for kind in LineKind::ALL {
            content.push_str(&self.lines((page, kind)));
        }
        content.push_str(&self.fold_marks(page));
        match page.tile {
            Some(tile) => content.push_str(&seams((window, tile))),
            None => content.push_str(&self.labels(page)),
        }
        content
    }

    /// The page's lines of one kind, each cut down to its view's window.
    fn lines(&self, (page, kind): (&TestPage, LineKind)) -> String {
        let mut commands = String::new();
        for view in &page.views {
            let Some(item) = self.items.get(view.item) else {
                continue;
            };
            for run in item
                .lines(kind)
                .iter()
                .flat_map(|line| line.clipped(view.window))
            {
                commands.push_str(&run_commands((&run, view)));
            }
        }
        match commands.is_empty() {
            true => commands,
            false => format!("{}\n{commands}", line_style(kind)),
        }
    }

    /// A numbered circle on each fold whose mark the page shows.
    fn fold_marks(&self, page: &TestPage) -> String {
        let mut content = String::new();
        for view in &page.views {
            let Some(item) = self.items.get(view.item) else {
                continue;
            };
            let spots = mark_spots((&item.folds, &self.seams_of(view.item), FOLD_MARK_RADIUS_MM));
            for (fold, spot) in item.folds.iter().zip(spots) {
                if inside((spot, view.window)) {
                    content.push_str(&fold_mark((view.to_page(spot), fold.step, fold.direction)));
                }
            }
        }
        content
    }

    /// Where item `item`'s tiles overlap, in its coordinates, for fold marks to keep out
    /// of (the next tile covers them, or the trim cuts them): none for a whole piece.
    fn seams_of(&self, item: usize) -> KeepOut {
        let mut seams = KeepOut {
            half_width_mm: HALF_OVERLAP_MM,
            ..KeepOut::default()
        };
        for page in &self.pages {
            let (Some(tile), Some(view)) = (page.tile, page.views.first()) else {
                continue;
            };
            if view.item != item {
                continue;
            }
            if tile.joins.right {
                seams.xs.push(view.window.max.x - HALF_OVERLAP_MM);
            }
            if tile.joins.bottom {
                seams.ys.push(view.window.max.y - HALF_OVERLAP_MM);
            }
        }
        seams
    }

    /// Each whole piece's label, just above it.
    fn labels(&self, page: &TestPage) -> String {
        let mut content = "0 g\n".to_owned();
        for view in &page.views {
            let Some(item) = self.items.get(view.item) else {
                continue;
            };
            let corner = view.to_page(Point::ORIGIN);
            content.push_str(&text_at(Text {
                at: Point::new(corner.x, corner.y - LABEL_ABOVE_MM),
                size_mm: LABEL_TEXT_MM,
                turn_deg: 0.0,
                font: Font::Regular,
                text: &item.label(),
            }));
        }
        content
    }

    /// A grey rule under the drawing, the page number and what the page shows, and the
    /// scale bar.
    fn footer(&self, footer: &Footer) -> String {
        let window = self.paper.window();
        let (_, height) = self.paper.size_mm();
        let baseline = height - MARGIN_MM - FOOTER_BASELINE_MM;
        let rule_y = window.max.y + RULE_BELOW_MM;
        let mut content = format!("{} G {} w [] 0 d\n", num(RULE_GREY), num(HATCH_LINE_MM));
        content.push_str(&line((
            Point::new(window.min.x, rule_y),
            Point::new(window.max.x, rule_y),
        )));
        content.push_str("0 g\n");
        content.push_str(&text_at(Text {
            at: Point::new(window.min.x, baseline),
            size_mm: FOOTER_TEXT_MM,
            turn_deg: 0.0,
            font: Font::Regular,
            text: &format!(
                "Page {} of {} - {}",
                footer.number, footer.total, footer.what
            ),
        }));
        content.push_str(&scale_bar((
            self.paper.scale_bar(),
            Point::new(window.max.x, baseline),
        )));
        content
    }
}

/// How each kind of line looks. Cut lines are black and solid, thick enough to follow
/// with scissors; creases blue and dashed, so they still read from a black-and-white
/// printer; perforations dotted.
fn line_style(kind: LineKind) -> &'static str {
    match kind {
        LineKind::Cut => "0 G 0.3 w [] 0 d",
        LineKind::Crease => "0.1 0.25 0.8 RG 0.25 w [2.5 1.5] 0 d",
        LineKind::Perforation => "0 G 0.25 w [0.2 1.3] 0 d",
    }
}

/// A run of points, moved onto the page and stroked.
fn run_commands((run, view): (&Polyline, &View)) -> String {
    let mut points = run.points.iter().map(|&point| view.to_page(point));
    let Some(first) = points.next() else {
        return String::new();
    };
    let mut commands = format!("{} {} m\n", num(first.x), num(first.y));
    for point in points {
        let _ = writeln!(commands, "{} {} l", num(point.x), num(point.y));
    }
    commands.push_str(match run.closed {
        true => "h S\n",
        false => "S\n",
    });
    commands
}

/// A fold's step number in a circle at `center`: a white circle for a fold away from the
/// printed side, a black one for a fold towards it.
pub(super) fn fold_mark((center, step, direction): (Point, u32, FoldDirection)) -> String {
    let (disc, ink) = match direction {
        FoldDirection::Mountain => ("1 g", "0 g"),
        FoldDirection::Valley => ("0 g", "1 g"),
    };
    let number = step.to_string();
    let width = text_width_mm((&number, FOLD_NUMBER_MM));
    format!(
        "q {disc} 0 G 0.2 w [] 0 d\n{}b\n{ink}\n{}Q\n",
        path_commands(&Path::circle(center, FOLD_MARK_RADIUS_MM)),
        text_at(Text {
            at: Point::new(
                center.x - width / 2.0,
                center.y + DIGIT_HALF_HEIGHT * FOLD_NUMBER_MM,
            ),
            size_mm: FOLD_NUMBER_MM,
            turn_deg: 0.0,
            font: Font::Bold,
            text: &number,
        }),
    )
}

/// "1 row", "3 columns".
pub(super) fn counted((count, noun): (usize, &str)) -> String {
    match count {
        1 => format!("1 {noun}"),
        _ => format!("{count} {noun}s"),
    }
}

/// The hatched strips to cut off a tile's left and top edges, where it meets a printed
/// tile.
fn trim_strips((window, tile): (Bounds, Tile)) -> String {
    let mut content = String::new();
    if tile.joins.left {
        content.push_str(&hatching(Bounds {
            min: window.min,
            max: Point::new(window.min.x + HALF_OVERLAP_MM, window.max.y),
        }));
    }
    if tile.joins.top {
        content.push_str(&hatching(Bounds {
            min: window.min,
            max: Point::new(window.max.x, window.min.y + HALF_OVERLAP_MM),
        }));
    }
    content
}

/// Grey lines at 45° across `strip`, clipped to it.
fn hatching(strip: Bounds) -> String {
    let mut content = format!(
        "q {} {} {} {} re W n {} G {} w [] 0 d\n",
        num(strip.min.x),
        num(strip.min.y),
        num(strip.length_mm()),
        num(strip.width_mm()),
        num(HATCH_GREY),
        num(HATCH_LINE_MM),
    );
    let lines = ((strip.length_mm() + strip.width_mm()) / HATCH_MM).ceil() as usize;
    for index in 0..=lines {
        // x + y is the same all along each line.
        let sum = strip.min.x + strip.min.y + index as f64 * HATCH_MM;
        content.push_str(&line((
            Point::new(sum - strip.min.y, strip.min.y),
            Point::new(sum - strip.max.y, strip.max.y),
        )));
    }
    content.push_str("Q\n");
    content
}

/// A side of a tile's drawing window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    Left,
    Right,
    Top,
    Bottom,
}

/// The marks on each side where a tile meets a printed neighbour. Its left and top seams
/// are cut along (dashed); its right and bottom seams are where the next tile's cut edge
/// goes (solid), with that tile's name.
fn seams((window, tile): (Bounds, Tile)) -> String {
    let mut content = String::new();
    if tile.joins.left {
        content.push_str(&seam((window, Side::Left, "cut this strip off")));
    }
    if tile.joins.top {
        content.push_str(&seam((window, Side::Top, "cut this strip off")));
    }
    if tile.joins.right {
        let next = tile_name((tile.column + 1, tile.row));
        content.push_str(&seam((window, Side::Right, &format!("{next} goes on top"))));
    }
    if tile.joins.bottom {
        let next = tile_name((tile.column, tile.row + 1));
        content.push_str(&seam((
            window,
            Side::Bottom,
            &format!("{next} goes on top"),
        )));
    }
    content
}

/// One seam: its line half an overlap in from `side`, diamonds across it, and `note`
/// along the strip between it and the window's edge.
fn seam((window, side, note): (Bounds, Side, &str)) -> String {
    let vertical = matches!(side, Side::Left | Side::Right);
    // Across the seam: where it runs, and the middle of its strip.
    let (at, strip_middle) = match side {
        Side::Left => (
            window.min.x + HALF_OVERLAP_MM,
            window.min.x + HALF_OVERLAP_MM / 2.0,
        ),
        Side::Right => (
            window.max.x - HALF_OVERLAP_MM,
            window.max.x - HALF_OVERLAP_MM / 2.0,
        ),
        Side::Top => (
            window.min.y + HALF_OVERLAP_MM,
            window.min.y + HALF_OVERLAP_MM / 2.0,
        ),
        Side::Bottom => (
            window.max.y - HALF_OVERLAP_MM,
            window.max.y - HALF_OVERLAP_MM / 2.0,
        ),
    };
    let (start, end) = match vertical {
        true => (window.min.y, window.max.y),
        false => (window.min.x, window.max.x),
    };
    let point = |(along, across): (f64, f64)| match vertical {
        true => Point::new(across, along),
        false => Point::new(along, across),
    };
    let dash = match side {
        Side::Left | Side::Top => {
            format!("[{} {}] 0 d", num(SEAM_DASH_MM[0]), num(SEAM_DASH_MM[1]))
        }
        Side::Right | Side::Bottom => "[] 0 d".to_owned(),
    };
    let mut content = format!("{} G {} w {dash}\n", num(SEAM_GREY), num(SEAM_LINE_MM));
    content.push_str(&line((point((start, at)), point((end, at)))));
    let _ = writeln!(content, "{} g", num(DIAMOND_GREY));
    for index in 0..DIAMONDS {
        let along = start + (end - start) * (f64::from(index) + 0.5) / f64::from(DIAMONDS);
        let corners = [
            point((along, at - DIAMOND_ACROSS_MM)),
            point((along - DIAMOND_ALONG_MM, at)),
            point((along, at + DIAMOND_ACROSS_MM)),
            point((along + DIAMOND_ALONG_MM, at)),
        ];
        let _ = writeln!(
            content,
            "{} {} m {} {} l {} {} l {} {} l h f",
            num(corners[0].x),
            num(corners[0].y),
            num(corners[1].x),
            num(corners[1].y),
            num(corners[2].x),
            num(corners[2].y),
            num(corners[3].x),
            num(corners[3].y),
        );
    }
    let width = text_width_mm((note, NOTE_MM));
    let middle_along = (start + end) / 2.0;
    // Reading upwards beside a vertical seam, left to right along a horizontal one; the
    // letters' middle on the strip's middle.
    let (at, turn_deg) = match vertical {
        true => (
            Point::new(
                strip_middle + DIGIT_HALF_HEIGHT * NOTE_MM,
                middle_along + width / 2.0,
            ),
            90.0,
        ),
        false => (
            Point::new(
                middle_along - width / 2.0,
                strip_middle + DIGIT_HALF_HEIGHT * NOTE_MM,
            ),
            0.0,
        ),
    };
    let _ = writeln!(content, "{} g", num(NOTE_GREY));
    content.push_str(&text_at(Text {
        at,
        size_mm: NOTE_MM,
        turn_deg,
        font: Font::Regular,
        text: note,
    }));
    content
}

/// The scale bar with its right end at `end`, standing on the footer's baseline, in
/// alternate black and white parts, with its label to its left.
fn scale_bar((bar, end): (ScaleBar, Point)) -> String {
    let start = end.x - bar.length_mm;
    let top = end.y - SCALE_BAR_MM;
    let part = bar.length_mm / f64::from(bar.parts);
    let mut content = format!("0 G 0 g {} w [] 0 d\n", num(HATCH_LINE_MM));
    for index in (0..bar.parts).step_by(2) {
        let _ = writeln!(
            content,
            "{} {} {} {} re f",
            num(start + f64::from(index) * part),
            num(top),
            num(part),
            num(SCALE_BAR_MM),
        );
    }
    let _ = writeln!(
        content,
        "{} {} {} {} re S",
        num(start),
        num(top),
        num(bar.length_mm),
        num(SCALE_BAR_MM),
    );
    let label = format!("check: this bar is {}", bar.label);
    let width = text_width_mm((&label, FOOTER_TEXT_MM));
    content.push_str(&text_at(Text {
        at: Point::new(start - SCALE_LABEL_GAP_MM - width, end.y),
        size_mm: FOOTER_TEXT_MM,
        turn_deg: 0.0,
        font: Font::Regular,
        text: &label,
    }));
    content
}

/// Rows of the watermark across the whole page, page `number`'s rows each shifted along
/// by their own amount.
fn watermark((paper, number): (Paper, usize)) -> String {
    let (width, height) = paper.size_mm();
    let center = Point::new(width / 2.0, height / 2.0);
    // Rows run from the centre out to the corners, whichever way they're turned.
    let reach = width.hypot(height) / 2.0;
    let phrase = format!("{WATERMARK}{WATERMARK_GAP}");
    let period = text_width_mm((&phrase, WATERMARK_MM));
    let row_text = phrase.repeat((2.0 * reach / period).ceil() as usize + 2);
    let (sin, cos) = WATERMARK_TURN_DEG.to_radians().sin_cos();
    let rows = (reach / WATERMARK_ROW_MM).ceil() as i64;
    let mut content = format!("q /GS1 gs {} g\n", num(WATERMARK_GREY));
    for row in -rows..=rows {
        // Along the row is (cos, -sin) on the page (y down); across it is (sin, cos).
        let across = row as f64 * WATERMARK_ROW_MM;
        let along = -reach - jitter((number as u64, row)) * period;
        content.push_str(&text_at(Text {
            at: Point::new(
                center.x + along * cos + across * sin,
                center.y - along * sin + across * cos,
            ),
            size_mm: WATERMARK_MM,
            turn_deg: WATERMARK_TURN_DEG,
            font: Font::Bold,
            text: &row_text,
        }));
    }
    content.push_str("Q\n");
    content
}

/// The whole file: catalog (asking readers to print at actual size), the page list,
/// Helvetica and Helvetica-Bold, the watermark's see-through setting, the document info,
/// then each page and its content.
fn document(((width, height), contents): ((f64, f64), &[String])) -> String {
    let page_object = |index: usize| FIRST_PAGE_OBJECT + 2 * index;
    let kids: Vec<String> = (0..contents.len())
        .map(|index| format!("{} 0 R", page_object(index)))
        .collect();
    let mut objects = vec![
        format!(
            "<< /Type /Catalog /Pages {PAGES_OBJECT} 0 R \
             /ViewerPreferences << /PrintScaling /None >> >>"
        ),
        format!(
            "<< /Type /Pages /Kids [{}] /Count {} >>",
            kids.join(" "),
            contents.len()
        ),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>"
            .to_owned(),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding >>"
            .to_owned(),
        format!(
            "<< /Type /ExtGState /ca {opacity} /CA {opacity} >>",
            opacity = num(WATERMARK_OPACITY)
        ),
        "<< /Title (Paccradle test print) /Producer (Paccradle insert_core) >>".to_owned(),
    ];
    for (index, content) in contents.iter().enumerate() {
        objects.push(format!(
            "<< /Type /Page /Parent {PAGES_OBJECT} 0 R /MediaBox [0 0 {} {}] \
             /Resources << /Font << /F1 {FONT_OBJECT} 0 R /F2 {BOLD_FONT_OBJECT} 0 R >> \
             /ExtGState << /GS1 {WATERMARK_STATE_OBJECT} 0 R >> >> /Contents {} 0 R >>",
            num(width * PT_PER_MM),
            num(height * PT_PER_MM),
            page_object(index) + 1,
        ));
        objects.push(format!(
            "<< /Length {} >>\nstream\n{content}endstream",
            content.len()
        ));
    }
    write_file((&objects, INFO_OBJECT))
}
