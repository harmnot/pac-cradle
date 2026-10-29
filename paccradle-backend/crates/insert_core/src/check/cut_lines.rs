//! Cut and crease lines (CLAUDE.md §6.5): cut lines that don't close, lines drawn twice
//! (die makers cut each line once), and creases that cross or end on a cut line away
//! from their own ends.
//!
//! A crease ends where its fold ends: on the outline, or at a slot's corner. Touching a
//! cut line there is how creases are drawn; anywhere else, the cut and the crease fight.

use std::collections::HashMap;

use super::{Check, Context, PieceRef, Rule, Warning, lines, points, union};
use crate::error::GeometryError;
use crate::geometry::{Bounds, Line, Point};

/// Lines whose directions differ by less than this (radians) run the same way.
const SAME_DIRECTION_RAD: f64 = 1e-6;
/// Parallel lines closer than this lie on one line.
const SAME_LINE_MM: f64 = 1e-4;
/// Overlaps and contacts shorter than this are a shared end point, not a fault.
const TOUCH_MM: f64 = 1e-3;

pub(super) struct CutLines;

impl Check for CutLines {
    fn run(&self, context: &Context) -> Result<Vec<Warning>, GeometryError> {
        let mut warnings = Vec::new();
        for piece in context.pieces() {
            warnings.extend(open_cuts(piece));
            let strokes = strokes(piece);
            warnings.extend(duplicates((piece, &strokes)));
            warnings.extend(crossings((piece, &strokes)));
        }
        Ok(warnings)
    }
}

/// One straight line of a piece's drawing.
#[derive(Clone, Copy)]
struct Stroke {
    /// Which path it belongs to: 0 for the outline, then the holes, then the creases.
    path: usize,
    line: Line,
    crease: bool,
}

/// Every straight line in the piece's cut and crease paths.
fn strokes(piece: PieceRef) -> Vec<Stroke> {
    let cuts = std::iter::once(&piece.piece.outline).chain(&piece.piece.holes);
    let paths = cuts
        .map(|path| (path, false))
        .chain(piece.piece.creases.iter().map(|path| (path, true)));
    paths
        .enumerate()
        .flat_map(|(path_index, (path, crease))| {
            lines(path).into_iter().map(move |line| Stroke {
                path: path_index,
                line,
                crease,
            })
        })
        .collect()
}

/// Cut paths (outline and holes) that don't close, or have too few points to.
fn open_cuts(piece: PieceRef<'_>) -> impl Iterator<Item = Warning> + '_ {
    std::iter::once(&piece.piece.outline)
        .chain(&piece.piece.holes)
        .filter_map(move |path| {
            let ring = points(path);
            let open = !path.closed || ring.points().len() < 3;
            open.then(|| Warning {
                rule: Rule::OpenCut,
                at: piece.spot().area(ring.bounds()),
            })
        })
}

/// A stroke placed on its endless line: direction angle (0 to π), offset of the line
/// from the origin, and where the stroke starts and ends along it.
struct OnLine {
    stroke: usize,
    angle: f64,
    offset: f64,
    span: (f64, f64),
}

fn on_line((index, line): (usize, Line)) -> Option<OnLine> {
    let (dx, dy) = (line.to.x - line.from.x, line.to.y - line.from.y);
    if dx.hypot(dy) < TOUCH_MM {
        return None;
    }
    let mut angle = dy.atan2(dx).rem_euclid(std::f64::consts::PI);
    if angle > std::f64::consts::PI - SAME_DIRECTION_RAD {
        angle -= std::f64::consts::PI;
    }
    let (sin, cos) = angle.sin_cos();
    let along = |point: Point| point.x * cos + point.y * sin;
    let (a, b) = (along(line.from), along(line.to));
    Some(OnLine {
        stroke: index,
        angle,
        offset: line.from.y * cos - line.from.x * sin,
        span: (a.min(b), a.max(b)),
    })
}

/// Lines drawn on top of each other: one warning per pair of paths, with the total
/// length they share.
fn duplicates((piece, strokes): (PieceRef, &[Stroke])) -> Vec<Warning> {
    let mut placed: Vec<OnLine> = strokes
        .iter()
        .enumerate()
        .filter_map(|(index, stroke)| on_line((index, stroke.line)))
        .collect();
    placed.sort_by(|a, b| {
        a.angle
            .total_cmp(&b.angle)
            .then(a.offset.total_cmp(&b.offset))
    });
    let mut shared = Shared::new();
    let same_direction = |a: &OnLine, b: &OnLine| b.angle - a.angle < SAME_DIRECTION_RAD;
    for direction in runs((&placed, same_direction)) {
        let mut by_offset: Vec<&OnLine> = direction.iter().collect();
        by_offset.sort_by(|a, b| a.offset.total_cmp(&b.offset));
        let same_line = |a: &&OnLine, b: &&OnLine| b.offset - a.offset < SAME_LINE_MM;
        for line in runs((&by_offset, same_line)) {
            overlaps((line, strokes, &mut shared));
        }
    }
    let mut pairs: Vec<_> = shared.into_iter().collect();
    pairs.sort_by_key(|&(pair, _)| pair);
    pairs
        .into_iter()
        .map(|(_, (length_mm, area))| Warning {
            rule: Rule::DuplicateLine { length_mm },
            at: piece.spot().area(area),
        })
        .collect()
}

/// Length and box of the lines each pair of paths shares, by (path, path).
type Shared = HashMap<(usize, usize), (f64, Option<Bounds>)>;

/// The sorted list cut into runs, each pair of neighbours in a run `together`.
fn runs<T>((sorted, together): (&[T], impl Fn(&T, &T) -> bool)) -> Vec<&[T]> {
    let mut runs = Vec::new();
    let mut start = 0;
    for end in 1..=sorted.len() {
        if end == sorted.len() || !together(&sorted[end - 1], &sorted[end]) {
            runs.push(&sorted[start..end]);
            start = end;
        }
    }
    runs
}

/// Adds every overlap between strokes on one line to `shared`, by pair of paths.
fn overlaps((on_one_line, strokes, shared): (&[&OnLine], &[Stroke], &mut Shared)) {
    let mut by_start: Vec<&OnLine> = on_one_line.to_vec();
    by_start.sort_by(|a, b| a.span.0.total_cmp(&b.span.0));
    for (index, first) in by_start.iter().enumerate() {
        for second in &by_start[index + 1..] {
            if second.span.0 >= first.span.1 - TOUCH_MM {
                break;
            }
            let length = first.span.1.min(second.span.1) - second.span.0;
            if length < TOUCH_MM {
                continue;
            }
            let (a, b) = (strokes[first.stroke], strokes[second.stroke]);
            let pair = (a.path.min(b.path), a.path.max(b.path));
            let entry = shared.entry(pair).or_insert((0.0, None));
            entry.0 += length;
            entry.1 = union(entry.1, Some(a.line.bounds().union(b.line.bounds())));
        }
    }
}

/// Creases that cross a cut line, or touch one away from the crease's own ends: one
/// warning per crease and cut path.
fn crossings((piece, strokes): (PieceRef, &[Stroke])) -> Vec<Warning> {
    let creases: Vec<&Stroke> = strokes.iter().filter(|stroke| stroke.crease).collect();
    let cuts: Vec<&Stroke> = strokes.iter().filter(|stroke| !stroke.crease).collect();
    let mut hits: HashMap<(usize, usize), Bounds> = HashMap::new();
    for crease in &creases {
        let ends = crease_ends((crease.path, strokes));
        for cut in &cuts {
            if crease.line.bounds().gap_to(cut.line.bounds()) > TOUCH_MM {
                continue;
            }
            let Some(point) = contact((crease.line, cut.line)) else {
                continue;
            };
            if ends.iter().any(|end| end.distance_to(point) < TOUCH_MM) {
                continue;
            }
            let spot = Bounds {
                min: point,
                max: point,
            };
            hits.entry((crease.path, cut.path))
                .and_modify(|bounds| *bounds = bounds.union(spot))
                .or_insert(spot);
        }
    }
    let mut hits: Vec<_> = hits.into_iter().collect();
    hits.sort_by_key(|&(pair, _)| pair);
    hits.into_iter()
        .map(|(_, bounds)| Warning {
            rule: Rule::CreaseCrossesCut,
            at: piece.spot().area(Some(bounds)),
        })
        .collect()
}

/// The two ends of the crease path `path`.
fn crease_ends((path, strokes): (usize, &[Stroke])) -> [Point; 2] {
    let on_path: Vec<&Stroke> = strokes
        .iter()
        .filter(|stroke| stroke.path == path)
        .collect();
    let first = on_path
        .first()
        .map_or(Point::ORIGIN, |stroke| stroke.line.from);
    let last = on_path.last().map_or(first, |stroke| stroke.line.to);
    [first, last]
}

/// Where two non-parallel lines meet, if they do. Lines that run the same way are left
/// to the duplicate rule.
fn contact((a, b): (Line, Line)) -> Option<Point> {
    let (adx, ady) = (a.to.x - a.from.x, a.to.y - a.from.y);
    let (bdx, bdy) = (b.to.x - b.from.x, b.to.y - b.from.y);
    let cross = adx * bdy - ady * bdx;
    let lengths = adx.hypot(ady) * bdx.hypot(bdy);
    if lengths == 0.0 || cross.abs() < SAME_DIRECTION_RAD * lengths {
        return None;
    }
    let (ex, ey) = (b.from.x - a.from.x, b.from.y - a.from.y);
    let along_a = (ex * bdy - ey * bdx) / cross;
    let along_b = (ex * ady - ey * adx) / cross;
    let slack_a = TOUCH_MM / adx.hypot(ady);
    let slack_b = TOUCH_MM / bdx.hypot(bdy);
    let within = |along: f64, slack: f64| (-slack..=1.0 + slack).contains(&along);
    (within(along_a, slack_a) && within(along_b, slack_b))
        .then(|| Point::new(a.from.x + along_a * adx, a.from.y + along_a * ady))
}
