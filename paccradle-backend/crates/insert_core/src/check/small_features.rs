//! Features smaller than 1 mm (CLAUDE.md §6.5): a strip of board, or a cut-out, narrower
//! than [`MIN_FEATURE_MM`]. Thin board tears, and two cuts that close can't be cut
//! cleanly: a die can't hold two rules that near, and a knife or laser merges them.
//!
//! Shrinking a shape by half the limit and growing it back loses exactly its parts
//! narrower than the limit; growing it and shrinking back fills exactly its gaps
//! narrower than the limit. Either way also rounds ordinary corners, by less than half
//! the limit, so parts shorter than the limit are left out. A sharp spike still counts:
//! its tip is thinner than the limit for longer than that.
//!
//! This runs on the outline alone (thin tabs and flaps, slits between panels, slots cut
//! in from the edge) and on each hole alone (a narrow slot, a spike of board poking into
//! the hole). Board left thin between two holes, or between a hole and the edge, is
//! already a hole spacing warning, whose limit (a web, at least 3 mm) is stricter.

use super::{Check, Context, MIN_FEATURE_MM, Rule, SmallFeature, Warning, points};
use crate::error::GeometryError;
use crate::geometry::{Area, Bounds, Line, Point, Polygon, Region};

/// Thinner than this, a part is floating-point noise along an edge, not a feature.
const NOISE_MM: f64 = 0.01;
/// Two edges this many times farther apart along the ring than across may enclose a
/// narrow part: the sides of a corner sharper than about 84°, or of a slot.
const FAR_ALONG: f64 = 1.5;

pub(super) struct SmallFeatures;

impl Check for SmallFeatures {
    fn run(&self, context: &Context) -> Result<Vec<Warning>, GeometryError> {
        let mut warnings = Vec::new();
        for piece in context.pieces() {
            // The outline's thin parts are board; its thin gaps are cut-outs.
            let outline_ring = points(&piece.piece.outline);
            let mut found = match needs_detail(&outline_ring) {
                true => thin_parts((
                    &Area::from_polygon(&outline_ring)?,
                    SmallFeature::Board,
                    SmallFeature::CutOut,
                ))?,
                false => Vec::new(),
            };
            // A hole's thin parts are cut-outs; its thin gaps are board poking into it.
            for hole in &piece.piece.holes {
                let ring = points(hole);
                if !needs_detail(&ring) {
                    continue;
                }
                let hole = Area::from_polygon(&ring)?;
                found.extend(thin_parts((
                    &hole,
                    SmallFeature::CutOut,
                    SmallFeature::Board,
                ))?);
            }
            warnings.extend(found.into_iter().map(|(feature, size_mm, bounds)| Warning {
                rule: Rule::TooSmall {
                    feature,
                    size_mm,
                    min_mm: MIN_FEATURE_MM,
                },
                at: piece.spot().area(Some(bounds)),
            }));
        }
        Ok(warnings)
    }
}

/// The parts of `shape` narrower than the limit (named `inside`), and the gaps in it
/// narrower than the limit (named `outside`): kind, rough width and box of each.
fn thin_parts(
    (shape, inside, outside): (&Area, SmallFeature, SmallFeature),
) -> Result<Vec<(SmallFeature, f64, Bounds)>, GeometryError> {
    let half = MIN_FEATURE_MM / 2.0;
    let thin = shape.difference(&shape.offset(-half)?.offset(half)?)?;
    let gaps = shape.offset(half)?.offset(-half)?.difference(shape)?;
    Ok([(inside, thin), (outside, gaps)]
        .iter()
        .flat_map(|(feature, area)| {
            area.regions().iter().filter_map(|region| {
                let bounds = region.outer.bounds()?;
                let long = bounds.length_mm().max(bounds.width_mm()) >= MIN_FEATURE_MM;
                let width = rough_width(region);
                (long && width > NOISE_MM).then_some((*feature, width, bounds))
            })
        })
        .collect())
}

/// About how wide a long, thin region is: twice its area over its perimeter.
fn rough_width(region: &Region) -> f64 {
    let perimeter: f64 = std::iter::once(&region.outer)
        .chain(&region.holes)
        .flat_map(|ring| ring.edges())
        .map(|edge| edge.from.distance_to(edge.to))
        .sum();
    match perimeter > 0.0 {
        true => 2.0 * region.area_mm2() / perimeter,
        false => 0.0,
    }
}

/// True when a ring may hold something narrower than the limit, so it needs the exact
/// shrink-and-grow: two stretches of it come closer than the limit while lying much
/// farther apart along the ring. That's the two sides of a narrow slot or spike, or of a
/// corner sharper than about 84°. Along a straight or gently curving edge, stretches are
/// never much farther apart along the ring than across. Edges are cut into stretches no
/// longer than the limit first, so a long slot's sides count too. Most rings pass, which
/// keeps a card of a hundred holes quick.
fn needs_detail(ring: &Polygon) -> bool {
    let mut stretches: Vec<(Line, Bounds)> = Vec::new();
    for edge in ring.edges() {
        let length = edge.from.distance_to(edge.to);
        // `as` saturates; validated pieces are at most a few metres round.
        let pieces = (length / MIN_FEATURE_MM).ceil().max(1.0) as usize;
        let at = |k: usize| {
            let share = k as f64 / pieces as f64;
            Point::new(
                edge.from.x + share * (edge.to.x - edge.from.x),
                edge.from.y + share * (edge.to.y - edge.from.y),
            )
        };
        stretches.extend((0..pieces).map(|k| {
            let line = Line {
                from: at(k),
                to: at(k + 1),
            };
            (line, line.bounds())
        }));
    }
    // How far round the ring each stretch starts.
    let mut along = Vec::with_capacity(stretches.len() + 1);
    let mut total = 0.0;
    for (line, _) in &stretches {
        along.push(total);
        total += line.from.distance_to(line.to);
    }
    along.push(total);
    let count = stretches.len();
    // Only stretches whose boxes come within the limit along x can be close.
    let mut by_x: Vec<usize> = (0..count).collect();
    by_x.sort_by(|&a, &b| stretches[a].1.min.x.total_cmp(&stretches[b].1.min.x));
    by_x.iter().enumerate().any(|(rank, &first)| {
        let reach = stretches[first].1.max.x + MIN_FEATURE_MM;
        by_x[rank + 1..]
            .iter()
            .take_while(|&&other| stretches[other].1.min.x < reach)
            .any(|&other| {
                let (i, j) = (first.min(other), first.max(other));
                if j == i + 1 || (i == 0 && j == count - 1) {
                    return false; // neighbours share a corner
                }
                let ((a, a_box), (b, b_box)) = (stretches[i], stretches[j]);
                if a_box.gap_to(b_box) >= MIN_FEATURE_MM {
                    return false;
                }
                let across = a.distance_to_line(b);
                let round = (along[j] - along[i + 1]).min(total - along[j + 1] + along[i]);
                across < MIN_FEATURE_MM && round > FAR_ALONG * across
            })
    })
}
