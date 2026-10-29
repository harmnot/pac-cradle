//! Drawing the laid-out preview, in the site's colours (the frontend's drawing tokens),
//! with the watermark over everything.

use super::PngError;
use super::canvas::{Canvas, Ink, Pen};
use super::font::{Font, Run, Weight};
use super::layout::{
    BODY_PX, Block, BlockKind, LEGEND_MARK_PX, LegendItem, LegendMark, Line, Preview,
    legend_text_offset,
};
use crate::export::marks::{KeepOut, mark_spots};
use crate::export::{LineKind, SheetPiece, jitter};
use crate::fit::FitColor;
use crate::geometry::{Bounds, CURVE_TOLERANCE_MM, Path, Point};
use crate::insert::Stance;
use crate::piece::FoldDirection;

/// The site's colours (`styles/tokens.css`).
const BACKGROUND: Ink = Ink::hex(0xfaf6ef); // label cream
const TEXT: Ink = Ink::hex(0x2b2219); // espresso ink
const TEXT_MUTED: Ink = Ink::hex(0x6b5f52);
const CUT: Ink = Ink::hex(0x2b2219);
const CREASE: Ink = Ink::hex(0x9a5236); // terracotta
const BOARD: Ink = Ink::hex(0xebdcc5); // kraft, soft
const BOARD_EDGE: Ink = Ink::hex(0xa68b6e); // kraft, deep
const FIT_OK: Ink = Ink::hex(0x4e8a5d);
const FIT_TIGHT: Ink = Ink::hex(0xc8962b);
const FIT_COLLIDE: Ink = Ink::hex(0xb23f2c);
/// How opaque a product's fill is, over the board under it.
const PRODUCT_FILL: f64 = 0.55;

/// Lines, in px.
const CUT_PEN: Pen = Pen::solid((CUT, 2.0));
const CREASE_PEN: Pen = Pen {
    ink: CREASE,
    width_px: 1.8,
    dash_px: Some([9.0, 6.0]),
};
const PERFORATION_PEN: Pen = Pen {
    ink: CUT,
    width_px: 1.8,
    dash_px: Some([1.0, 5.0]),
};
const BOARD_PEN: Pen = Pen::solid((BOARD_EDGE, 1.5));
const BOX_PEN: Pen = Pen {
    ink: BOARD_EDGE,
    width_px: 2.0,
    dash_px: Some([12.0, 7.0]),
};
const PRODUCT_LINE_PX: f64 = 2.0;

/// Fold marks: a circle this big round the step number.
const FOLD_MARK_RADIUS_PX: f64 = 11.0;
const FOLD_NUMBER_PX: f64 = 14.0;
const FOLD_MARK_PEN: Pen = Pen::solid((TEXT, 1.5));
/// Digits stand about this share of the font size above the baseline, halved: how far
/// below the middle a centred number's baseline goes.
const DIGIT_HALF_HEIGHT: f64 = 0.36;

/// The watermark: rows of bold text across the whole picture, turned, see-through, drawn
/// over the drawings. Each design shifts its rows its own way, so the pattern isn't the
/// same from one picture to the next.
const WATERMARK: &str = "PACCRADLE PREVIEW - NOT FOR PRODUCTION";
const WATERMARK_GAP: &str = "      ";
const WATERMARK_INK: Ink = TEXT.alpha(0.13);
/// Its size: the picture's longest side over this, kept between the two limits.
const WATERMARK_SIZE_SHARE: f64 = 55.0;
const WATERMARK_MIN_PX: f64 = 22.0;
const WATERMARK_MAX_PX: f64 = 44.0;
/// Rows this many text sizes apart.
const WATERMARK_ROW_SIZES: f64 = 4.0;
const WATERMARK_TURN_DEG: f64 = 30.0;

impl Preview {
    /// The preview as a PNG file.
    pub fn to_png(&self) -> Result<Vec<u8>, PngError> {
        let regular = Font::new(Weight::Regular)?;
        let bold = Font::new(Weight::Bold)?;
        let fonts = Fonts {
            regular: &regular,
            bold: &bold,
        };
        let mut canvas = Canvas::new(((self.width_px, self.height_px), BACKGROUND))?;
        for (index, line) in self.header.iter().enumerate() {
            let ink = match index {
                0 => TEXT,
                _ => TEXT_MUTED,
            };
            fonts.line((&mut canvas, (line, ink)));
        }
        for block in &self.blocks {
            fonts.line((&mut canvas, (&block.label, TEXT)));
            match block.kind {
                BlockKind::TopView => self.top_view((&mut canvas, block)),
                BlockKind::Piece(index) => {
                    if let Some(piece) = self.parts.sheet.pieces.get(index) {
                        self.piece((&mut canvas, (block, piece), &fonts));
                    }
                }
            }
        }
        for item in &self.legend {
            legend_item((&mut canvas, item, &fonts));
        }
        fonts.line((&mut canvas, (&self.footer, TEXT_MUTED)));
        self.watermark((&mut canvas, &bold));
        canvas.into_png()
    }

    /// The insert from above in the box's inside: boards, then products by fit.
    fn top_view(&self, (canvas, block): (&mut Canvas, &Block)) {
        let to_px = |point: Point| self.to_px((block, point));
        let ring = |points: &[Point], shift: Point| -> Vec<Point> {
            points
                .iter()
                .map(|point| to_px(Point::new(point.x + shift.x, point.y + shift.y)))
                .collect()
        };
        if let Some(inside) = self.parts.inside {
            canvas.line(((&rectangle(inside).map(to_px), true), BOX_PEN));
        }
        let insert = &self.parts.insert;
        for mount in &insert.mounts {
            let Some(piece) = insert.pieces.get(mount.piece) else {
                continue;
            };
            let [x, y, _] = mount.at_mm;
            let at = Point::new(x, y);
            let rings: Vec<Vec<Point>> = match mount.stance {
                // Lying flat: its root panel (the card, or a cradle's deck), holes and all.
                Stance::Flat => piece
                    .panels
                    .first()
                    .map(|root| {
                        std::iter::once(&root.outline)
                            .chain(&root.holes)
                            .map(|ring_mm| ring(ring_mm.points(), at))
                            .collect()
                    })
                    .unwrap_or_default(),
                // Standing on edge: a strip one board thick along its length.
                Stance::UprightAlongX | Stance::UprightAlongY => upright_from_above((
                    piece_bounds(&piece.outline),
                    mount.stance,
                    (at, self.parts.thickness_mm),
                ))
                .map(|strip| vec![rectangle(strip).map(to_px).to_vec()])
                .unwrap_or_default(),
            };
            canvas.fill((&rings, BOARD));
            for ring in &rings {
                canvas.line(((ring, true), BOARD_PEN));
            }
        }
        for (outline, color) in &self.parts.products {
            let ink = fit_ink(*color);
            let ring = ring(outline.points(), Point::ORIGIN);
            canvas.fill((std::slice::from_ref(&ring), ink.alpha(PRODUCT_FILL)));
            canvas.line(((&ring, true), Pen::solid((ink, PRODUCT_LINE_PX))));
        }
    }

    /// One piece flat: creases, perforations and cut lines, then its fold marks.
    fn piece(&self, (canvas, (block, piece), fonts): (&mut Canvas, (&Block, &SheetPiece), &Fonts)) {
        let to_px = |point: Point| self.to_px((block, point));
        for (kind, pen) in [
            (LineKind::Crease, CREASE_PEN),
            (LineKind::Perforation, PERFORATION_PEN),
            (LineKind::Cut, CUT_PEN),
        ] {
            for path in piece.lines(kind) {
                let points: Vec<Point> = path
                    .flatten(CURVE_TOLERANCE_MM)
                    .into_iter()
                    .map(to_px)
                    .collect();
                canvas.line(((&points, path.closed), pen));
            }
        }
        let radius_mm = FOLD_MARK_RADIUS_PX / block.scale;
        let spots = mark_spots((&piece.folds, &KeepOut::default(), radius_mm));
        for (fold, spot) in piece.folds.iter().zip(spots) {
            fold_mark((canvas, (to_px(spot), fold.step, fold.direction), fonts));
        }
    }

    /// Rows of the watermark across the whole picture.
    fn watermark(&self, (canvas, font): (&mut Canvas, &Font)) {
        let (width, height) = (f64::from(self.width_px), f64::from(self.height_px));
        let size =
            (width.max(height) / WATERMARK_SIZE_SHARE).clamp(WATERMARK_MIN_PX, WATERMARK_MAX_PX);
        let center = Point::new(width / 2.0, height / 2.0);
        // Rows run from the centre out to the corners, whichever way they're turned.
        let reach = width.hypot(height) / 2.0;
        let phrase = format!("{WATERMARK}{WATERMARK_GAP}");
        let period = font.width_px((&phrase, size));
        if period <= 0.0 {
            return;
        }
        let row_text = phrase.repeat((2.0 * reach / period).ceil() as usize + 2);
        let (sin, cos) = WATERMARK_TURN_DEG.to_radians().sin_cos();
        let spacing = size * WATERMARK_ROW_SIZES;
        let rows = (reach / spacing).ceil() as i64;
        let seed = self.seed();
        for row in -rows..=rows {
            // Along the row is (cos, -sin) in the picture (y down); across it is (sin, cos).
            let across = row as f64 * spacing;
            let along = -reach - jitter((seed, row)) * period;
            let run = Run {
                text: &row_text,
                at: Point::new(
                    center.x + along * cos + across * sin,
                    center.y - along * sin + across * cos,
                ),
                size_px: size,
                turn_deg: WATERMARK_TURN_DEG,
            };
            canvas.text(((font, run), WATERMARK_INK));
        }
    }

    /// A number from the pieces' sizes, the same for the same design, to shift the
    /// watermark's rows by (FNV-1a).
    fn seed(&self) -> u64 {
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        for piece in &self.parts.sheet.pieces {
            let bounds = piece.bounds;
            for value in [bounds.min.x, bounds.min.y, bounds.max.x, bounds.max.y] {
                for byte in value.to_bits().to_le_bytes() {
                    hash ^= u64::from(byte);
                    hash = hash.wrapping_mul(0x0100_0000_01b3);
                }
            }
        }
        hash
    }
}

/// The two weights the preview writes in.
struct Fonts<'a> {
    regular: &'a Font,
    bold: &'a Font,
}

impl Fonts<'_> {
    fn line(&self, (canvas, (line, ink)): (&mut Canvas, (&Line, Ink))) {
        let font = match line.bold {
            true => self.bold,
            false => self.regular,
        };
        let run = Run {
            text: &line.text,
            at: line.at,
            size_px: line.size_px,
            turn_deg: 0.0,
        };
        canvas.text(((font, run), ink));
    }
}

fn fit_ink(color: FitColor) -> Ink {
    match color {
        FitColor::Green => FIT_OK,
        FitColor::Yellow => FIT_TIGHT,
        FitColor::Red => FIT_COLLIDE,
    }
}

/// A fold's step number in a circle: white for a fold away from the side shown, dark for
/// one towards it.
fn fold_mark(
    (canvas, (center, step, direction), fonts): (&mut Canvas, (Point, u32, FoldDirection), &Fonts),
) {
    let (disc, ink) = match direction {
        FoldDirection::Mountain => (Ink::hex(0xffffff), TEXT),
        FoldDirection::Valley => (TEXT, BACKGROUND),
    };
    canvas.disc(((center, FOLD_MARK_RADIUS_PX), (disc, FOLD_MARK_PEN)));
    let number = step.to_string();
    let width = fonts.bold.width_px((&number, FOLD_NUMBER_PX));
    let run = Run {
        text: &number,
        at: Point::new(
            center.x - width / 2.0,
            center.y + DIGIT_HALF_HEIGHT * FOLD_NUMBER_PX,
        ),
        size_px: FOLD_NUMBER_PX,
        turn_deg: 0.0,
    };
    canvas.text(((fonts.bold, run), ink));
}

/// A legend entry: its mark, then its words.
fn legend_item((canvas, item, fonts): (&mut Canvas, &LegendItem, &Fonts)) {
    // The mark sits on the middle of the words' letters.
    let middle = item.at.y - DIGIT_HALF_HEIGHT * BODY_PX;
    let (left, right) = (item.at.x, item.at.x + LEGEND_MARK_PX);
    match item.mark {
        LegendMark::Fit(color) => {
            let half = BODY_PX * 0.4;
            let square = rectangle(Bounds {
                min: Point::new(left + 4.0, middle - half),
                max: Point::new(right - 4.0, middle + half),
            });
            let ink = fit_ink(color);
            canvas.fill((&[square.to_vec()], ink.alpha(PRODUCT_FILL)));
            canvas.line(((&square, true), Pen::solid((ink, PRODUCT_LINE_PX))));
        }
        LegendMark::Cut => canvas.line((
            (
                &[Point::new(left, middle), Point::new(right, middle)],
                false,
            ),
            CUT_PEN,
        )),
        LegendMark::Crease => canvas.line((
            (
                &[Point::new(left, middle), Point::new(right, middle)],
                false,
            ),
            CREASE_PEN,
        )),
        LegendMark::FoldStep => fold_mark((
            canvas,
            (
                Point::new((left + right) / 2.0, middle),
                1,
                FoldDirection::Mountain,
            ),
            fonts,
        )),
    }
    let run = Run {
        text: item.text,
        at: Point::new(item.at.x + legend_text_offset(), item.at.y),
        size_px: BODY_PX,
        turn_deg: 0.0,
    };
    canvas.text(((fonts.regular, run), TEXT));
}

/// A box's corners, clockwise from the top-left.
fn rectangle(bounds: Bounds) -> [Point; 4] {
    [
        bounds.min,
        Point::new(bounds.max.x, bounds.min.y),
        bounds.max,
        Point::new(bounds.min.x, bounds.max.y),
    ]
}

fn piece_bounds(outline: &Path) -> Option<Bounds> {
    Bounds::of(outline.flatten(CURVE_TOLERANCE_MM))
}

/// A piece standing on edge, seen from above: a strip one board thick along its length,
/// in the insert's frame.
fn upright_from_above(
    (bounds, stance, (at, thickness)): (Option<Bounds>, Stance, (Point, f64)),
) -> Option<Bounds> {
    let bounds = bounds?;
    let half = thickness / 2.0;
    Some(match stance {
        Stance::UprightAlongY => Bounds {
            min: Point::new(at.x - half, at.y + bounds.min.x),
            max: Point::new(at.x + half, at.y + bounds.max.x),
        },
        _ => Bounds {
            min: Point::new(at.x + bounds.min.x, at.y - half),
            max: Point::new(at.x + bounds.max.x, at.y + half),
        },
    })
}
