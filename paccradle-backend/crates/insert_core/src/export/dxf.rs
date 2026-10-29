//! DXF production file (M1-B3, CLAUDE.md §6.6): for die makers' CAD, laser and CNC
//! software, and cutters that take DXF.
//!
//! - DXF R12 (AC1009), the version nearly every program reads (LibreCAD, Inkscape,
//!   LightBurn, Silhouette Studio, ArtiosCAD).
//! - Units are mm: `$INSUNITS` 4 and `$MEASUREMENT` 1 in the header. y points up, as
//!   in CAD, so the drawing is flipped onto the sheet's bottom-left corner.
//! - Layers CUT (red, colour 1), CREASE (blue, 5) and PERFORATION (green, 3; only when a
//!   piece has perforations), all continuous lines.
//! - Every path is one `POLYLINE`, closed where the path is, so a cutter follows it in one
//!   go. Arcs are bulges on the polyline's vertices, at most a quarter turn each.
//! - No text: a cutter would cut it. How many of each piece to cut is in the PDF.
//! - Written by hand: R12 is plain text in pairs of lines (a group code, then a value).

use std::fmt::Write;

use super::num;
use super::sheet::{LineKind, Sheet};
use crate::geometry::{Path, Point, Segment};

/// AutoCAD's colour numbers for the layers.
const RED: u8 = 1;
const GREEN: u8 = 3;
const BLUE: u8 = 5;

/// `$INSUNITS` for millimetres.
const MILLIMETRES: u8 = 4;

/// Arcs are split into parts no longer than this; a bulge must stay finite.
const MAX_ARC_PART_DEG: f64 = 90.0;

/// Points closer than this are the same point.
const SAME_POINT_MM: f64 = 1e-9;

/// The `70` flag on a closed polyline.
const CLOSED: u8 = 1;

impl Sheet {
    /// The sheet as a DXF R12 file.
    pub fn to_dxf(&self) -> String {
        let kinds = self.used_kinds();
        let mut dxf = Dxf::default();
        dxf.header((self.length_mm, self.width_mm));
        dxf.tables(&kinds);
        dxf.section("ENTITIES");
        for kind in kinds {
            for path in self.pieces.iter().flat_map(|piece| piece.lines(kind)) {
                dxf.polyline((path, kind, self.width_mm));
            }
        }
        dxf.pair((0, "ENDSEC"));
        dxf.pair((0, "EOF"));
        dxf.text
    }

    /// The line kinds to write: PERFORATION only when a piece has some.
    fn used_kinds(&self) -> Vec<LineKind> {
        let perforated = self
            .pieces
            .iter()
            .any(|piece| !piece.perforations.is_empty());
        LineKind::ALL
            .into_iter()
            .filter(|&kind| kind != LineKind::Perforation || perforated)
            .collect()
    }
}

/// A DXF file being written: group codes and values, a line each.
#[derive(Default)]
struct Dxf {
    text: String,
}

impl Dxf {
    /// One group: its code right-aligned in three places, as DXF writers do, then the value.
    fn pair(&mut self, (code, value): (u16, &str)) {
        let _ = write!(self.text, "{code:>3}\n{value}\n");
    }

    fn number(&mut self, (code, value): (u16, f64)) {
        self.pair((code, &num(value)));
    }

    fn integer(&mut self, (code, value): (u16, i64)) {
        self.pair((code, &value.to_string()));
    }

    fn section(&mut self, name: &str) {
        self.pair((0, "SECTION"));
        self.pair((2, name));
    }

    /// A point's x, y and z groups (10, 20, 30, or 11, 21, 31 and so on).
    fn point(&mut self, (code, point): (u16, Point)) {
        self.number((code, point.x));
        self.number((code + 10, point.y));
        self.number((code + 20, 0.0));
    }

    /// The version, the units and the drawing's extent.
    fn header(&mut self, (length_mm, width_mm): (f64, f64)) {
        self.section("HEADER");
        self.pair((9, "$ACADVER"));
        self.pair((1, "AC1009"));
        self.pair((9, "$INSUNITS"));
        self.integer((70, i64::from(MILLIMETRES)));
        self.pair((9, "$MEASUREMENT"));
        self.integer((70, 1));
        self.pair((9, "$EXTMIN"));
        self.point((10, Point::ORIGIN));
        self.pair((9, "$EXTMAX"));
        self.point((10, Point::new(length_mm, width_mm)));
        self.pair((0, "ENDSEC"));
    }

    /// The continuous line type and one layer per line kind, coloured.
    fn tables(&mut self, kinds: &[LineKind]) {
        self.section("TABLES");
        self.table(("LTYPE", 1));
        self.pair((0, "LTYPE"));
        self.pair((2, "CONTINUOUS"));
        self.integer((70, 0));
        self.pair((3, "Solid line"));
        self.integer((72, 65));
        self.integer((73, 0));
        self.number((40, 0.0));
        self.pair((0, "ENDTAB"));

        self.table(("LAYER", kinds.len()));
        for &kind in kinds {
            self.pair((0, "LAYER"));
            self.pair((2, kind.layer_name()));
            self.integer((70, 0));
            self.integer((62, i64::from(color(kind))));
            self.pair((6, "CONTINUOUS"));
        }
        self.pair((0, "ENDTAB"));
        self.pair((0, "ENDSEC"));
    }

    fn table(&mut self, (name, entries): (&str, usize)) {
        self.pair((0, "TABLE"));
        self.pair((2, name));
        self.pair((70, &entries.to_string()));
    }

    /// One path as a polyline on its kind's layer, y flipped up from the sheet's bottom.
    fn polyline(&mut self, (path, kind, sheet_width_mm): (&Path, LineKind, f64)) {
        let layer = kind.layer_name();
        let flip = |point: Point| Point::new(point.x, sheet_width_mm - point.y);
        self.pair((0, "POLYLINE"));
        self.pair((8, layer));
        self.integer((66, 1));
        self.point((10, Point::ORIGIN));
        self.integer((70, i64::from(if path.closed { CLOSED } else { 0 })));
        for vertex in vertices(path) {
            self.pair((0, "VERTEX"));
            self.pair((8, layer));
            self.point((10, flip(vertex.at)));
            if vertex.bulge != 0.0 {
                self.number((42, vertex.bulge));
            }
        }
        self.pair((0, "SEQEND"));
        self.pair((8, layer));
    }
}

fn color(kind: LineKind) -> u8 {
    match kind {
        LineKind::Cut => RED,
        LineKind::Crease => BLUE,
        LineKind::Perforation => GREEN,
    }
}

/// A polyline vertex: where it is, and the bulge of the stretch to the next vertex (0 for
/// a straight line).
#[derive(Debug, Clone, Copy, PartialEq)]
struct Vertex {
    at: Point,
    bulge: f64,
}

/// The path's vertices with bulges. A closed polyline joins its last vertex back to the
/// first by itself, so a path that ends where it started drops that last point.
fn vertices(path: &Path) -> Vec<Vertex> {
    let mut vertices = vec![Vertex {
        at: path.start,
        bulge: 0.0,
    }];
    for segment in &path.segments {
        match *segment {
            Segment::Line { to } => vertices.push(Vertex { at: to, bulge: 0.0 }),
            Segment::Arc { center, sweep_deg } => {
                let from = vertices.last().map_or(path.start, |vertex| vertex.at);
                for (end, bulge) in arc_parts((from, center, sweep_deg)) {
                    // A bulge belongs to the vertex its stretch starts from.
                    if let Some(last) = vertices.last_mut() {
                        last.bulge = bulge;
                    }
                    vertices.push(Vertex {
                        at: end,
                        bulge: 0.0,
                    });
                }
            }
        }
    }
    let ends_at_start = vertices.len() > 1
        && vertices
            .last()
            .is_some_and(|last| last.at.distance_to(path.start) <= SAME_POINT_MM);
    if path.closed && ends_at_start {
        vertices.pop();
    }
    vertices
}

/// An arc split into parts of at most [`MAX_ARC_PART_DEG`]: each part's end and bulge.
///
/// DXF measures a bulge with y up, where positive turns anticlockwise. The sheet's y
/// points down, where a positive sweep turns clockwise on screen. Flipping y keeps the
/// picture, so a positive sweep is a clockwise, negative bulge.
fn arc_parts((start, center, sweep_deg): (Point, Point, f64)) -> Vec<(Point, f64)> {
    let parts = (sweep_deg.abs() / MAX_ARC_PART_DEG).ceil().max(1.0) as u32;
    let part_deg = sweep_deg / f64::from(parts);
    let bulge = -(part_deg.to_radians() / 4.0).tan();
    let radius = start.distance_to(center);
    let first_angle = (start.y - center.y).atan2(start.x - center.x);
    (1..=parts)
        .map(|part| {
            let angle = first_angle + (part_deg * f64::from(part)).to_radians();
            let end = Point::new(
                center.x + radius * angle.cos(),
                center.y + radius * angle.sin(),
            );
            (end, bulge)
        })
        .collect()
}
