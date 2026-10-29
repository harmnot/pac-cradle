//! Text for the PNG preview, in Geist: the site's own typeface, bundled so the server
//! needs no installed fonts (SIL Open Font License, `fonts/OFL.txt`).

use tiny_skia::{Path, PathBuilder};
use ttf_parser::{Face, GlyphId, OutlineBuilder, Tag};

use super::PngError;
use crate::geometry::Point;

/// Geist as a variable font: one file for every weight.
const GEIST: &[u8] = include_bytes!("../../../fonts/Geist-Variable.ttf");

/// Stands in for a character the font lacks.
const MISSING: char = '?';

/// A weight on the font's `wght` axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Weight {
    Regular,
    Bold,
}

impl Weight {
    fn axis_value(self) -> f32 {
        match self {
            Weight::Regular => 400.0,
            Weight::Bold => 700.0,
        }
    }
}

/// Geist at one weight.
pub(super) struct Font {
    face: Face<'static>,
}

/// One run of text.
pub(super) struct Run<'a> {
    pub text: &'a str,
    /// Where the baseline starts, in px (y down).
    pub at: Point,
    pub size_px: f64,
    /// How far the text turns anticlockwise: 0 reads left to right, 90 upwards.
    pub turn_deg: f64,
}

impl Font {
    pub(super) fn new(weight: Weight) -> Result<Font, PngError> {
        let mut face = Face::parse(GEIST, 0).map_err(|_| PngError::Font)?;
        face.set_variation(Tag::from_bytes(b"wght"), weight.axis_value())
            .ok_or(PngError::Font)?;
        Ok(Font { face })
    }

    /// How wide `text` sets at `size_px`.
    pub(super) fn width_px(&self, (text, size_px): (&str, f64)) -> f64 {
        let units: f64 = text
            .chars()
            .filter_map(|character| self.glyph(character))
            .map(|glyph| self.advance(glyph))
            .sum();
        units * self.px_per_unit(size_px)
    }

    /// The letters' outlines, ready to fill; `None` for text with nothing to draw.
    pub(super) fn path(&self, run: Run) -> Option<Path> {
        let (sin, cos) = run.turn_deg.to_radians().sin_cos();
        let scale = self.px_per_unit(run.size_px);
        let mut builder = PathBuilder::new();
        let mut pen = 0.0;
        for glyph in run
            .text
            .chars()
            .filter_map(|character| self.glyph(character))
        {
            let mut outline = Outline {
                builder: &mut builder,
                place: Place {
                    at: run.at,
                    pen,
                    scale,
                    sin,
                    cos,
                },
            };
            let _ = self.face.outline_glyph(glyph, &mut outline);
            pen += self.advance(glyph) * scale;
        }
        builder.finish()
    }

    fn px_per_unit(&self, size_px: f64) -> f64 {
        size_px / f64::from(self.face.units_per_em())
    }

    fn glyph(&self, character: char) -> Option<GlyphId> {
        self.face
            .glyph_index(character)
            .or_else(|| self.face.glyph_index(MISSING))
    }

    fn advance(&self, glyph: GlyphId) -> f64 {
        self.face.glyph_hor_advance(glyph).map_or(0.0, f64::from)
    }
}

/// Where a glyph's points land: font units (y up) from the pen, scaled to px, turned,
/// then moved to the run's start.
#[derive(Clone, Copy)]
struct Place {
    at: Point,
    /// How far along the run the glyph starts, in px.
    pen: f64,
    scale: f64,
    sin: f64,
    cos: f64,
}

impl Place {
    fn point(self, (x, y): (f32, f32)) -> (f32, f32) {
        let along = self.pen + f64::from(x) * self.scale;
        let down = -f64::from(y) * self.scale;
        // Anticlockwise on a page whose y runs down.
        let px = self.at.x + along * self.cos + down * self.sin;
        let py = self.at.y - along * self.sin + down * self.cos;
        (px as f32, py as f32)
    }
}

/// Copies a glyph's outline into a path, placed.
struct Outline<'a> {
    builder: &'a mut PathBuilder,
    place: Place,
}

impl OutlineBuilder for Outline<'_> {
    fn move_to(&mut self, x: f32, y: f32) {
        let (x, y) = self.place.point((x, y));
        self.builder.move_to(x, y);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let (x, y) = self.place.point((x, y));
        self.builder.line_to(x, y);
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let (x1, y1) = self.place.point((x1, y1));
        let (x, y) = self.place.point((x, y));
        self.builder.quad_to(x1, y1, x, y);
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let (x1, y1) = self.place.point((x1, y1));
        let (x2, y2) = self.place.point((x2, y2));
        let (x, y) = self.place.point((x, y));
        self.builder.cubic_to(x1, y1, x2, y2, x, y);
    }

    fn close(&mut self) {
        self.builder.close();
    }
}
