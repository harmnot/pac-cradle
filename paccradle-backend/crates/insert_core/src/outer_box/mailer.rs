//! Mailer box, FEFCO 0427 style ("roll end, tuck front"): one piece, no glue.
//!
//! - The base is the root. A single back wall and a single front wall fold up from it,
//!   each with a dust flap at both ends that folds in along the box's ends.
//! - Each end is double: the outer end wall folds up from the base, a rim rolls over the
//!   top, and the inner end wall comes down inside, trapping the dust flaps between the
//!   two. Lock tabs on the inner wall's bottom edge go down through slots in the base and
//!   end flush with its underside.
//! - The lid folds over from the top of the back wall. Its tuck flap slides down between
//!   the front wall and the inside.
//!
//! In the box's frame (inside from (0, 0, 0) to (L, W, H); t the board, g its slot
//! tolerance), the middle planes sit at:
//! - base z = −t/2, lid z = H + t/2;
//! - back wall y = −t/2, tuck flap y = W + t/2, front wall y = W + 1.5t + g;
//! - inner end wall x = −t/2, dust flaps x = −1.5t, outer end wall x = −2.5t − g, so the
//!   rim is 2t + g across (and the mirror image at the right end).
//!
//! Nothing reaches into the inside. As on a real tuck-front mailer, the front wall's top
//! edge shows in front of the lid's fold.

use super::{BoxConstruction, BuiltBox};
use crate::design::BoxSize;
use crate::error::GenerateError;
use crate::folding::{BuiltPiece, FoldingPiece, Joint, LockTabs, along_x, along_y, rect};
use crate::geometry::{Area, Line, Point, Polygon};
use crate::insert::{Mount, Stance};
use crate::material::Material;
use crate::piece::FoldDirection;

/// Name of the piece in files and the editor.
const PIECE_NAME: &str = "mailer";
/// Deepest tuck flap; shallower in a low box (half the height).
const MAX_TUCK_MM: f64 = 30.0;
/// Every crease folds a quarter turn.
const FOLD_DEG: f64 = 90.0;

/// The mailer construction.
pub(super) struct Mailer;

/// Fold order for the animation, the order the box is put together in.
#[derive(Debug, Clone, Copy)]
enum Stage {
    /// Back and front walls up.
    Walls,
    DustFlaps,
    OuterEnds,
    Rims,
    InnerEnds,
    Lid,
    Tuck,
}

/// The left or right end of the box. The left end is drawn, the right end is its mirror
/// image across the middle of the inside.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum End {
    Left,
    Right,
}

impl End {
    const BOTH: [Self; 2] = [Self::Left, Self::Right];

    fn x(self, (x, length_mm): (f64, f64)) -> f64 {
        match self {
            Self::Left => x,
            Self::Right => length_mm - x,
        }
    }

    fn point(self, (point, length_mm): (Point, f64)) -> Point {
        Point::new(self.x((point.x, length_mm)), point.y)
    }

    fn polygon(self, (polygon, length_mm): (Polygon, f64)) -> Polygon {
        Polygon(
            polygon
                .points()
                .iter()
                .map(|&point| self.point((point, length_mm)))
                .collect(),
        )
    }

    fn line(self, (line, length_mm): (Line, f64)) -> Line {
        Line {
            from: self.point((line.from, length_mm)),
            to: self.point((line.to, length_mm)),
        }
    }
}

/// The lock tabs on the inner end walls, along the box's width.
fn lock_tabs(material: &Material) -> LockTabs {
    LockTabs {
        web_mm: material.min_web_mm(),
        tolerance_mm: material.slot_tolerance_mm,
    }
}

impl BoxConstruction for Mailer {
    /// The inner end walls take at least one lock tab, and the dust flaps are at least a
    /// web tall.
    fn min_inside(&self, material: &Material) -> BoxSize {
        let web = material.min_web_mm();
        BoxSize {
            length_mm: web,
            width_mm: lock_tabs(material).min_edge_mm(),
            height_mm: material.thickness_mm + 2.0 * material.slot_tolerance_mm + web,
        }
    }

    fn build(&self, (inside, material): (BoxSize, &Material)) -> Result<BuiltBox, GenerateError> {
        let too_small = || {
            let min = self.min_inside(material);
            GenerateError::BoxTooSmall {
                min_length_mm: min.length_mm,
                min_width_mm: min.width_mm,
                min_height_mm: min.height_mm,
            }
        };
        let tabs = lock_tabs(material)
            .spans(inside.width_mm)
            .ok_or_else(too_small)?;
        let plan = Plan {
            t: material.thickness_mm,
            g: material.slot_tolerance_mm,
            inside,
        };
        let built = plan.piece(&tabs)?;
        let t = plan.t;
        let g = plan.g;
        Ok(BuiltBox {
            piece: built.piece,
            outside_mm: BoxSize {
                length_mm: inside.length_mm + 6.0 * t + 2.0 * g,
                width_mm: inside.width_mm + 3.0 * t + g,
                height_mm: inside.height_mm + 2.0 * t,
            },
            mount: Mount {
                piece: 0,
                stance: Stance::Flat,
                at_mm: [built.flat_min.x, built.flat_min.y, -t],
            },
        })
    }
}

/// Where everything goes, in the box's frame. The base is drawn as it lies (x and y as
/// in the box), and the other panels unfold from it.
struct Plan {
    /// Board thickness.
    t: f64,
    /// Slot tolerance: the play left where board slides past board.
    g: f64,
    inside: BoxSize,
}

impl Plan {
    fn length(&self) -> f64 {
        self.inside.length_mm
    }

    /// The left outer end wall's crease, on the base.
    fn outer_x(&self) -> f64 {
        -(2.5 * self.t + self.g)
    }

    /// The crease between the back (or front) wall and its left dust flap.
    fn flap_x(&self) -> f64 {
        -1.5 * self.t
    }

    /// The left inner end wall's middle plane, once folded.
    fn inner_x(&self) -> f64 {
        -self.t / 2.0
    }

    /// The back wall's crease, on the base.
    fn back_y(&self) -> f64 {
        -self.t / 2.0
    }

    /// The front wall's crease, on the base.
    fn front_y(&self) -> f64 {
        self.inside.width_mm + 1.5 * self.t + self.g
    }

    /// How far a dust flap reaches in along the end: the back and front flaps meet in
    /// the middle with the tolerance between them.
    fn dust_depth(&self) -> f64 {
        self.inside.width_mm / 2.0 + self.t
    }

    /// The dust flaps' height range: clear of the base and of the rim by the tolerance.
    fn dust_z(&self) -> (f64, f64) {
        (self.g, self.inside.height_mm - self.t - self.g)
    }

    /// The whole flat piece.
    fn piece(&self, tabs: &[(f64, f64)]) -> Result<BuiltPiece, GenerateError> {
        let (t, length, height) = (self.t, self.length(), self.inside.height_mm);
        let width = self.inside.width_mm;
        let slots: Vec<Polygon> = End::BOTH
            .iter()
            .flat_map(|&end| {
                self.slots(tabs)
                    .map(move |slot| end.polygon((slot, length)))
            })
            .collect();
        let base = rect((
            (self.outer_x(), length - self.outer_x()),
            (self.back_y(), self.front_y()),
        ));
        let base = Area::from_polygon(&base)?.difference(&Area::from_polygons(&slots)?)?;
        let mut piece = FoldingPiece::new((PIECE_NAME, base));
        let walls_across = (self.flap_x(), length - self.flap_x());

        // Back wall, from the base up to the lid's crease: z runs from −t/2 to H + t/2,
        // drawn upwards (y = −t − z).
        let back_z = |z: f64| -t - z;
        let lid_y = back_z(height + t / 2.0);
        let back = piece.fold((
            Area::from_polygon(&rect((walls_across, (self.back_y(), lid_y))))?,
            self.joint((0, along_x((self.back_y(), walls_across))), Stage::Walls),
        ));
        let lid_across = (self.outer_x() - t / 2.0, length - self.outer_x() + t / 2.0);
        let tuck_y = lid_y - (width + t);
        let lid = piece.fold((
            Area::from_polygon(&rect((lid_across, (lid_y, tuck_y))))?,
            self.joint((back, along_x((lid_y, walls_across))), Stage::Lid),
        ));
        let tuck_depth = (height / 2.0).min(MAX_TUCK_MM);
        piece.fold((
            Area::from_polygon(&rect(((0.0, length), (tuck_y, tuck_y - tuck_depth))))?,
            self.joint((lid, along_x((tuck_y, (0.0, length)))), Stage::Tuck),
        ));
        let (low, high) = self.dust_z();
        self.dust_flaps((&mut piece, back, (back_z(high), back_z(low))))?;

        // Front wall, from the base up to the lid's underside: drawn downwards.
        let front_z = |z: f64| self.front_y() + t / 2.0 + z;
        let front = piece.fold((
            Area::from_polygon(&rect((walls_across, (self.front_y(), front_z(height)))))?,
            self.joint((0, along_x((self.front_y(), walls_across))), Stage::Walls),
        ));
        self.dust_flaps((&mut piece, front, (front_z(low), front_z(high))))?;

        for end in End::BOTH {
            self.end_walls((&mut piece, end, tabs))?;
        }
        piece.build()
    }

    fn joint(&self, (parent, axis): (usize, Line), stage: Stage) -> Joint {
        Joint {
            parent,
            axis,
            deg: FOLD_DEG,
            direction: FoldDirection::Valley,
            offset_mm: 0.0,
            stage: stage as u32,
        }
    }

    /// A dust flap at both ends of the wall `parent`, hinged along its end edges between
    /// `ys` (drawn y values).
    fn dust_flaps(
        &self,
        (piece, parent, ys): (&mut FoldingPiece, usize, (f64, f64)),
    ) -> Result<(), GenerateError> {
        let length = self.length();
        let flap = rect(((self.flap_x() - self.dust_depth(), self.flap_x()), ys));
        let axis = along_y((self.flap_x(), ys));
        for end in End::BOTH {
            piece.fold((
                Area::from_polygon(&end.polygon((flap.clone(), length)))?,
                self.joint((parent, end.line((axis, length))), Stage::DustFlaps),
            ));
        }
        Ok(())
    }

    /// One end's outer wall, rim and inner wall with its lock tabs.
    fn end_walls(
        &self,
        (piece, end, tabs): (&mut FoldingPiece, End, &[(f64, f64)]),
    ) -> Result<(), GenerateError> {
        let (t, g, length) = (self.t, self.g, self.length());
        let (height, width) = (self.inside.height_mm, self.inside.width_mm);
        let place = |polygon: Polygon| Area::from_polygon(&end.polygon((polygon, length)));
        let line = |line: Line| end.line((line, length));

        // Outer wall: from the base up to the rim's crease, H between the creases.
        let outer_x = self.outer_x();
        let rim_x = outer_x - height;
        let full_width = (self.back_y(), self.front_y());
        let outer = piece.fold((
            place(rect(((outer_x, rim_x), full_width)))?,
            self.joint((0, line(along_y((outer_x, full_width)))), Stage::OuterEnds),
        ));
        // Rim and inner wall run between the back wall and the tuck flap.
        let inside_width = (0.0, width);
        let inner_x = rim_x - (2.0 * t + g);
        let rim = piece.fold((
            place(rect(((rim_x, inner_x), inside_width)))?,
            self.joint((outer, line(along_y((rim_x, inside_width)))), Stage::Rims),
        ));
        // Inner wall: from the rim down to the base's top, then the tabs through it.
        let bottom_x = inner_x - (height - t / 2.0);
        let mut inner = vec![rect(((inner_x, bottom_x), inside_width))];
        inner.extend(
            tabs.iter()
                .map(|&(start, end)| rect(((bottom_x, bottom_x - t), (start, end)))),
        );
        let inner: Vec<Polygon> = inner
            .into_iter()
            .map(|polygon| end.polygon((polygon, length)))
            .collect();
        piece.fold((
            Area::from_polygons(&inner)?,
            self.joint(
                (rim, line(along_y((inner_x, inside_width)))),
                Stage::InnerEnds,
            ),
        ));
        Ok(())
    }

    /// The base's lock slots at the left end: one per tab, a board plus the tolerance
    /// wide, under the inner end wall.
    fn slots<'a>(&'a self, tabs: &'a [(f64, f64)]) -> impl Iterator<Item = Polygon> + 'a {
        let half_width = (self.t + self.g) / 2.0;
        let half_tolerance = self.g / 2.0;
        let across = (self.inner_x() - half_width, self.inner_x() + half_width);
        tabs.iter().map(move |&(start, end)| {
            rect((across, (start - half_tolerance, end + half_tolerance)))
        })
    }
}
