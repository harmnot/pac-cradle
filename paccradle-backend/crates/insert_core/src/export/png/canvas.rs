//! The PNG preview's pixels, drawn with tiny-skia: lines, filled shapes and text, in px
//! (x right, y down), always smoothed.

use tiny_skia::{
    FillRule, LineCap, LineJoin, Paint, Path, PathBuilder, Pixmap, Stroke, StrokeDash, Transform,
};

use super::PngError;
use super::font::{Font, Run};
use crate::geometry::Point;

/// A colour, and how opaque it is (0–1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Ink {
    pub rgb: u32,
    pub alpha: f64,
}

impl Ink {
    /// `0xRRGGBB`, fully opaque.
    pub(super) const fn hex(rgb: u32) -> Ink {
        Ink { rgb, alpha: 1.0 }
    }

    /// The same colour at `alpha`.
    pub(super) const fn alpha(self, alpha: f64) -> Ink {
        Ink { alpha, ..self }
    }

    fn paint(self) -> Paint<'static> {
        let [_, red, green, blue] = self.rgb.to_be_bytes();
        let mut paint = Paint::default();
        paint.set_color_rgba8(
            red,
            green,
            blue,
            (self.alpha.clamp(0.0, 1.0) * 255.0).round() as u8,
        );
        paint.anti_alias = true;
        paint
    }
}

/// How a line is drawn.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Pen {
    pub ink: Ink,
    pub width_px: f64,
    /// On and off lengths, for a dashed line.
    pub dash_px: Option<[f64; 2]>,
}

impl Pen {
    pub(super) const fn solid((ink, width_px): (Ink, f64)) -> Pen {
        Pen {
            ink,
            width_px,
            dash_px: None,
        }
    }

    fn stroke(self) -> Stroke {
        Stroke {
            width: self.width_px as f32,
            line_cap: LineCap::Round,
            line_join: LineJoin::Round,
            dash: self
                .dash_px
                .and_then(|[on, off]| StrokeDash::new(vec![on as f32, off as f32], 0.0)),
            ..Stroke::default()
        }
    }
}

pub(super) struct Canvas {
    pixmap: Pixmap,
}

impl Canvas {
    /// A blank `width` × `height` px picture in `background`.
    pub(super) fn new(
        ((width, height), background): ((u32, u32), Ink),
    ) -> Result<Canvas, PngError> {
        let mut pixmap = Pixmap::new(width, height).ok_or(PngError::Size { width, height })?;
        let [_, red, green, blue] = background.rgb.to_be_bytes();
        pixmap.fill(tiny_skia::Color::from_rgba8(red, green, blue, 255));
        Ok(Canvas { pixmap })
    }

    /// A line through `points`, joined back to the first when `closed`.
    pub(super) fn line(&mut self, ((points, closed), pen): ((&[Point], bool), Pen)) {
        if let Some(path) = polyline((points, closed)) {
            self.pixmap.stroke_path(
                &path,
                &pen.ink.paint(),
                &pen.stroke(),
                Transform::identity(),
                None,
            );
        }
    }

    /// Closed rings filled even-odd, so a ring inside another is a hole.
    pub(super) fn fill(&mut self, (rings, ink): (&[Vec<Point>], Ink)) {
        let mut builder = PathBuilder::new();
        for ring in rings {
            add_points((&mut builder, (ring, true)));
        }
        if let Some(path) = builder.finish() {
            self.fill_path((&path, ink));
        }
    }

    /// Text in `font`.
    pub(super) fn text(&mut self, ((font, run), ink): ((&Font, Run), Ink)) {
        if let Some(path) = font.path(run) {
            self.fill_path((&path, ink));
        }
    }

    /// A circle, filled and outlined.
    pub(super) fn disc(&mut self, ((center, radius_px), (fill, pen)): ((Point, f64), (Ink, Pen))) {
        let circle = PathBuilder::from_circle(center.x as f32, center.y as f32, radius_px as f32);
        if let Some(path) = circle {
            self.fill_path((&path, fill));
            self.pixmap.stroke_path(
                &path,
                &pen.ink.paint(),
                &pen.stroke(),
                Transform::identity(),
                None,
            );
        }
    }

    fn fill_path(&mut self, (path, ink): (&Path, Ink)) {
        self.pixmap.fill_path(
            path,
            &ink.paint(),
            FillRule::EvenOdd,
            Transform::identity(),
            None,
        );
    }

    /// The picture as a PNG file.
    pub(super) fn into_png(self) -> Result<Vec<u8>, PngError> {
        self.pixmap
            .encode_png()
            .map_err(|error| PngError::Encode(error.to_string()))
    }
}

/// A path through `points`; `None` with fewer than two.
fn polyline((points, closed): (&[Point], bool)) -> Option<Path> {
    let mut builder = PathBuilder::new();
    add_points((&mut builder, (points, closed)));
    builder.finish()
}

fn add_points((builder, (points, closed)): (&mut PathBuilder, (&[Point], bool))) {
    let mut points = points.iter();
    let Some(first) = points.next() else {
        return;
    };
    builder.move_to(first.x as f32, first.y as f32);
    for point in points {
        builder.line_to(point.x as f32, point.y as f32);
    }
    if closed {
        builder.close();
    }
}
