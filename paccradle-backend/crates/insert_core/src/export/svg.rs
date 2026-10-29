//! SVG production file (M1-B1, CLAUDE.md §6.6): for laser and CNC makers and cutter
//! apps such as Cricut Design Space.
//!
//! - One layer (`<g>`) per line kind: CUT in red, CREASE in blue, PERFORATION in green
//!   (only when a piece has perforations). Inkscape shows them as layers; cutter apps
//!   pick an operation per colour.
//! - Inside each layer, one group per piece, titled with its name and how many to cut.
//!   The file has no drawn text, so a cutter never cuts a label.
//! - Units are mm: the canvas is sized in mm and one user unit is one mm.
//! - Hairline strokes (0.025 mm, about 0.001"): lasers that cut only hairlines (Epilog)
//!   cut these instead of engraving them.
//! - Arcs stay arcs; one longer than a half turn is written as two halves.

use std::fmt::Write;

use super::num;
use super::sheet::{LineKind, Sheet, SheetPiece};
use crate::check::Part;
use crate::geometry::{Path, Point, Segment, Transform};

/// Line width in mm: a hairline for lasers.
const STROKE_MM: f64 = 0.025;

/// The longest arc one SVG arc command draws without ambiguity.
const HALF_TURN_DEG: f64 = 180.0;

impl Sheet {
    /// The sheet as an SVG file.
    pub fn to_svg(&self) -> String {
        let (length, width) = (num(self.length_mm), num(self.width_mm));
        let mut svg = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <svg xmlns=\"http://www.w3.org/2000/svg\" \
             xmlns:inkscape=\"http://www.inkscape.org/namespaces/inkscape\" version=\"1.1\" \
             width=\"{length}mm\" height=\"{width}mm\" viewBox=\"0 0 {length} {width}\">\n\
             <title>Paccradle dieline</title>\n\
             <desc>Units: mm. CUT (red): cut lines. CREASE (blue): crease lines. \
             PERFORATION (green, when present): perforated cuts. Each piece is drawn once; \
             its title says how many to cut. Box pieces are mirrored so their print side \
             faces up.</desc>\n"
        );
        for kind in LineKind::ALL {
            let pieces: Vec<&SheetPiece> = self
                .pieces
                .iter()
                .filter(|piece| !piece.lines(kind).is_empty())
                .collect();
            if kind == LineKind::Perforation && pieces.is_empty() {
                continue;
            }
            svg.push_str(&layer((kind, &pieces)));
        }
        svg.push_str("</svg>\n");
        svg
    }
}

/// One line kind's layer: a group per piece.
fn layer((kind, pieces): (LineKind, &[&SheetPiece])) -> String {
    let name = kind.layer_name();
    let mut svg = format!(
        "<g id=\"{name}\" inkscape:groupmode=\"layer\" inkscape:label=\"{name}\" \
         fill=\"none\" stroke=\"{}\" stroke-width=\"{}\">\n",
        kind.color(),
        num(STROKE_MM),
    );
    for piece in pieces {
        let _ = writeln!(
            svg,
            "<g id=\"{}-{}-{}\">\n<title>{}</title>",
            name.to_lowercase(),
            part_id(piece.part),
            piece.index,
            escape(&piece.title()),
        );
        for path in piece.lines(kind) {
            let _ = writeln!(svg, "<path d=\"{}\"/>", path_data(path));
        }
        svg.push_str("</g>\n");
    }
    svg.push_str("</g>\n");
    svg
}

fn part_id(part: Part) -> &'static str {
    match part {
        Part::Insert => "insert",
        Part::OuterBox => "box",
    }
}

/// SVG path data, keeping arcs as arcs.
fn path_data(path: &Path) -> String {
    let mut data = format!("M{} {}", num(path.start.x), num(path.start.y));
    let mut current = path.start;
    for segment in &path.segments {
        match *segment {
            Segment::Line { to } => {
                let _ = write!(data, " L{} {}", num(to.x), num(to.y));
                current = to;
            }
            Segment::Arc { center, sweep_deg } => {
                let parts = if sweep_deg.abs() > HALF_TURN_DEG {
                    2
                } else {
                    1
                };
                let part_deg = sweep_deg / f64::from(parts);
                let radius = num(current.distance_to(center));
                // Positive sweeps turn clockwise on screen (y down): SVG's sweep flag 1.
                let sweep_flag = u8::from(part_deg > 0.0);
                for _ in 0..parts {
                    current = turned((current, center, part_deg));
                    let _ = write!(
                        data,
                        " A{radius} {radius} 0 0 {sweep_flag} {} {}",
                        num(current.x),
                        num(current.y)
                    );
                }
            }
        }
    }
    if path.closed {
        data.push_str(" Z");
    }
    data
}

/// `point` turned by `deg` about `center`.
fn turned((point, center, deg): (Point, Point, f64)) -> Point {
    Transform::shift(Point::ORIGIN - center)
        .then(Transform {
            turn_deg: deg,
            shift: center,
        })
        .apply(point)
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
