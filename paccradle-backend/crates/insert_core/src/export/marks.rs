//! Where fold step numbers go on a piece. The test print (B4) and the PNG preview (B5)
//! both put a numbered circle on each fold's crease.

use super::Fold;
use crate::geometry::{Line, Point};

/// A mark moves at most this many spacings along its crease, either way (far enough to
/// leave a test print's overlap), and stays this share of the crease's length from its
/// ends.
const TRIES: i32 = 6;
const END_SHARE: f64 = 0.1;
/// Marks keep two radii plus this share of one apart, centre to centre: creases a board
/// apart (a mailer's double end walls) would otherwise stack their circles.
const GAP_SHARE: f64 = 0.25;

/// Lines marks keep away from, with a band either side: where a test print's tiles
/// overlap. Empty for a piece drawn whole.
#[derive(Debug, Clone, Default)]
pub(crate) struct KeepOut {
    /// Lines running down the piece, at these x.
    pub xs: Vec<f64>,
    /// Lines running across, at these y.
    pub ys: Vec<f64>,
    /// How far either side of each line the band reaches.
    pub half_width_mm: f64,
}

impl KeepOut {
    /// True when a mark of `radius_mm` at `spot` stays wholly out of every band.
    fn clear_of(&self, (spot, radius_mm): (Point, f64)) -> bool {
        let reach = self.half_width_mm + radius_mm;
        self.xs.iter().all(|x| (spot.x - x).abs() >= reach)
            && self.ys.iter().all(|y| (spot.y - y).abs() >= reach)
    }
}

/// Where each fold's mark of `radius_mm` goes, in the folds' coordinates: the middle of
/// its crease, or the nearest spot along it that keeps clear of the marks placed before
/// and of `keep_out`. When no spot clears `keep_out` too, the nearest one clear of the
/// other marks. Worked out per piece, so a mark lands in the same place wherever the
/// piece is drawn.
pub(crate) fn mark_spots((folds, keep_out, radius_mm): (&[Fold], &KeepOut, f64)) -> Vec<Point> {
    let spacing = 2.0 * radius_mm + GAP_SHARE * radius_mm;
    let mut spots: Vec<Point> = Vec::with_capacity(folds.len());
    for fold in folds {
        let Line { from, to } = fold.axis;
        let length = from.distance_to(to);
        let at = |share: f64| {
            Point::new(
                from.x + share * (to.x - from.x),
                from.y + share * (to.y - from.y),
            )
        };
        let step = match length > 0.0 {
            true => spacing / length,
            false => 0.0,
        };
        // The middle, then a spacing either side, then two, …
        let candidates: Vec<Point> = (0..=2 * TRIES)
            .map(|try_| {
                let side = match try_ % 2 {
                    0 => -1.0,
                    _ => 1.0,
                };
                0.5 + side * f64::from((try_ + 1) / 2) * step
            })
            .filter(|share| (END_SHARE..=1.0 - END_SHARE).contains(share))
            .map(at)
            .collect();
        let apart = |spot: &Point| {
            spots
                .iter()
                .all(|placed| placed.distance_to(*spot) >= spacing)
        };
        let spot = candidates
            .iter()
            .find(|spot| apart(spot) && keep_out.clear_of((**spot, radius_mm)))
            .or_else(|| candidates.iter().find(|spot| apart(spot)))
            .copied()
            .unwrap_or_else(|| at(0.5));
        spots.push(spot);
    }
    spots
}
