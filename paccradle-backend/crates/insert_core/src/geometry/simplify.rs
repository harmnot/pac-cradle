//! Drops needless corners from outlines (Ramer–Douglas–Peucker).

use std::iter;

use super::{Line, Point, Polygon};

impl Polygon {
    /// Drops every corner the outline can lose without moving more than `tolerance_mm`
    /// (Ramer–Douglas–Peucker): each dropped point lies within `tolerance_mm` of the
    /// edge that replaces it. A tolerance of 0 drops only duplicate and in-line points.
    ///
    /// Polygons of 3 points or fewer come back unchanged. A polygon thinner than
    /// `tolerance_mm` collapses to fewer than 3 points, so check before using the result
    /// as an outline.
    pub fn simplify(&self, tolerance_mm: f64) -> Polygon {
        let points = self.points();
        if points.len() <= 3 {
            return self.clone();
        }
        // Split the ring at its first point and the point farthest from it; both always stay.
        let first = points[0];
        let farthest = (1..points.len())
            .max_by(|&a, &b| {
                first
                    .distance_to(points[a])
                    .total_cmp(&first.distance_to(points[b]))
            })
            .unwrap_or(0);
        let there: Vec<usize> = (0..=farthest).collect();
        let back: Vec<usize> = (farthest..points.len()).chain(iter::once(0)).collect();

        let mut keep = vec![false; points.len()];
        keep[0] = true;
        keep[farthest] = true;
        let tolerance_mm = tolerance_mm.max(0.0);
        for chain in [&there, &back] {
            let run = Run {
                points,
                chain,
                tolerance_mm,
            };
            run.mark_corners(&mut keep);
        }
        let kept = points
            .iter()
            .zip(&keep)
            .filter(|(_, keep)| **keep)
            .map(|(point, _)| *point);
        Polygon(kept.collect())
    }
}

/// One open run of an outline: `chain` lists indices into `points`, first to last.
struct Run<'a> {
    points: &'a [Point],
    chain: &'a [usize],
    tolerance_mm: f64,
}

impl Run<'_> {
    /// Sets `keep` for each point of the run that must stay. Works through a stack, not
    /// recursion, so long outlines can't overflow the call stack.
    fn mark_corners(&self, keep: &mut [bool]) {
        let mut spans = vec![(0, self.chain.len().saturating_sub(1))];
        while let Some((first, last)) = spans.pop() {
            if last <= first + 1 {
                continue;
            }
            let edge = Line {
                from: self.points[self.chain[first]],
                to: self.points[self.chain[last]],
            };
            let (worst, distance) = (first + 1..last)
                .map(|at| (at, edge.distance_to(self.points[self.chain[at]])))
                .fold((first, f64::NEG_INFINITY), |best, next| {
                    if next.1 > best.1 { next } else { best }
                });
            if distance <= self.tolerance_mm {
                continue;
            }
            keep[self.chain[worst]] = true;
            spans.push((first, worst));
            spans.push((worst, last));
        }
    }
}
