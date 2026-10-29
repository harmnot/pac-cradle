//! Cutting lines down to the part of a piece one page shows, so a tiled page carries
//! only its own part of the drawing (not the whole piece hidden behind a clip).

use super::{Polyline, inside};
use crate::geometry::{Bounds, Point};

/// Points closer than this join into one run.
const SAME_POINT_MM: f64 = 1e-9;

impl Polyline {
    /// The parts of this line inside `window`, edges included, each a run of points. A
    /// closed line wholly inside stays closed.
    pub(super) fn clipped(&self, window: Bounds) -> Vec<Polyline> {
        if self.points.iter().all(|&point| inside((point, window))) {
            return vec![self.clone()];
        }
        let count = self.points.len();
        let segments = match self.closed {
            true => count,
            false => count.saturating_sub(1),
        };
        let mut runs: Vec<Vec<Point>> = Vec::new();
        for index in 0..segments {
            let segment = (self.points[index], self.points[(index + 1) % count]);
            let Some((from, to)) = clip_segment((segment, window)) else {
                continue;
            };
            match runs.last_mut() {
                Some(run)
                    if run
                        .last()
                        .is_some_and(|&end| end.distance_to(from) <= SAME_POINT_MM) =>
                {
                    run.push(to);
                }
                _ => runs.push(vec![from, to]),
            }
        }
        join_round((&mut runs, self.closed));
        runs.into_iter()
            .map(|points| Polyline {
                points,
                closed: false,
            })
            .collect()
    }
}

/// A closed line that starts inside the window: its last run carries on into its first,
/// so they become one.
fn join_round((runs, closed): (&mut Vec<Vec<Point>>, bool)) {
    if !closed || runs.len() < 2 {
        return;
    }
    let start = runs.first().and_then(|run| run.first()).copied();
    let end = runs.last().and_then(|run| run.last()).copied();
    let wraps = match (start, end) {
        (Some(start), Some(end)) => start.distance_to(end) <= SAME_POINT_MM,
        _ => false,
    };
    if !wraps {
        return;
    }
    if let Some(mut last) = runs.pop() {
        last.extend(runs[0].iter().skip(1));
        runs[0] = last;
    }
}

/// The part of a straight segment inside `window` (Liang–Barsky), or `None` when it
/// misses or only touches a corner.
fn clip_segment(((from, to), window): ((Point, Point), Bounds)) -> Option<(Point, Point)> {
    let (dx, dy) = (to.x - from.x, to.y - from.y);
    let (mut enter, mut leave) = (0.0_f64, 1.0_f64);
    // For each edge: how fast the segment heads out through it, and how far inside the
    // start is.
    let edges = [
        (-dx, from.x - window.min.x),
        (dx, window.max.x - from.x),
        (-dy, from.y - window.min.y),
        (dy, window.max.y - from.y),
    ];
    for (outward, room) in edges {
        if outward == 0.0 {
            if room < 0.0 {
                return None;
            }
            continue;
        }
        let at = room / outward;
        match outward < 0.0 {
            true => enter = enter.max(at),
            false => leave = leave.min(at),
        }
    }
    let point_at = |t: f64| Point::new(from.x + t * dx, from.y + t * dy);
    (enter < leave).then(|| (point_at(enter), point_at(leave)))
}
