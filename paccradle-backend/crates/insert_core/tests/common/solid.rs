//! Folded board as solid blocks, to check that panels don't run into each other and to
//! measure what they enclose. Every fold in the boxes and the cradle is a quarter turn,
//! so each folded panel lies square to an axis, and its board is a union of axis-aligned
//! blocks: one board thick, centred on the middle plane.

use insert_core::{Hinge, Point, Polygon};

use super::fold::{FoldedPanel, V3};
use super::measure::{edges, is_inside};

/// Board that overlaps by no more than this is touching, not colliding: the polygon
/// library snaps to 0.1 µm.
pub const TOUCH_MM: f64 = 1e-3;

/// An axis-aligned block, in mm.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Block {
    pub min: V3,
    pub max: V3,
}

impl Block {
    /// How far the two blocks overlap along x, y and z (negative: the gap between them).
    pub fn overlap(&self, other: &Block) -> V3 {
        [0, 1, 2]
            .map(|axis| self.max[axis].min(other.max[axis]) - self.min[axis].max(other.min[axis]))
    }

    /// True when the blocks share more than a touching face.
    pub fn collides(&self, other: &Block) -> bool {
        self.overlap(other)
            .iter()
            .all(|&overlap| overlap > TOUCH_MM)
    }
}

/// One folded panel's board.
pub struct Solid {
    /// The axis the panel's face looks along: 0, 1 or 2 for x, y or z.
    pub normal: usize,
    pub blocks: Vec<Block>,
}

/// The two axes a panel facing along `normal` spans, in order.
pub fn in_plane(normal: usize) -> [usize; 2] {
    match normal {
        0 => [1, 2],
        1 => [0, 2],
        _ => [0, 1],
    }
}

/// True when every edge of the ring runs along an axis.
fn is_square(ring: &Polygon) -> bool {
    edges(ring).all(|edge| {
        (edge.to.x - edge.from.x).abs() <= TOUCH_MM || (edge.to.y - edge.from.y).abs() <= TOUCH_MM
    })
}

/// The ring's corners, sorted along one axis, with values closer than touching merged.
fn cut_lines((rings, pick): (&[&Polygon], fn(&Point) -> f64)) -> Vec<f64> {
    let mut values: Vec<f64> = rings
        .iter()
        .flat_map(|ring| ring.points())
        .map(pick)
        .collect();
    values.sort_by(f64::total_cmp);
    values.dedup_by(|later, earlier| (*later - *earlier).abs() <= TOUCH_MM);
    values
}

/// One panel's board: the part that turned, and its bend if it has one.
fn solid((index, panel, thickness_mm): (usize, &FoldedPanel, f64)) -> Result<Solid, String> {
    let (normal, mut blocks) = flat_blocks((index, &panel.outline, &panel.holes, thickness_mm))?;
    if let Some(bend) = &panel.bend {
        let (_, bend_blocks) = flat_blocks((index, bend, &[], thickness_mm))?;
        blocks.extend(bend_blocks);
    }
    Ok(Solid { normal, blocks })
}

/// The blocks of one flat ring of board with its holes, and the axis it faces along.
/// Fails when the ring doesn't lie square to an axis (a fold that isn't a quarter turn),
/// or when it has an edge that doesn't run along one.
fn flat_blocks(
    (index, outline, holes, thickness_mm): (usize, &[V3], &[Vec<V3>], f64),
) -> Result<(usize, Vec<Block>), String> {
    let spread = |axis: usize| {
        let values = outline.iter().map(|p| p[axis]);
        values.clone().fold(f64::NEG_INFINITY, f64::max) - values.fold(f64::INFINITY, f64::min)
    };
    let normal = (0..3)
        .find(|&axis| spread(axis) <= TOUCH_MM)
        .ok_or(format!("panel {index} isn't square to an axis"))?;
    let plane = outline[0][normal];
    let [u_axis, v_axis] = in_plane(normal);
    let flatten = |ring: &[V3]| {
        Polygon(
            ring.iter()
                .map(|p| Point::new(p[u_axis], p[v_axis]))
                .collect(),
        )
    };
    let outline = flatten(outline);
    if !is_square(&outline) {
        return Err(format!("panel {index} has an edge that isn't square"));
    }
    let holes: Vec<Polygon> = holes
        .iter()
        .map(|ring| flatten(ring))
        .filter(is_square)
        .collect();
    let rings: Vec<&Polygon> = std::iter::once(&outline).chain(&holes).collect();
    let us = cut_lines((&rings, |p| p.x));
    let vs = cut_lines((&rings, |p| p.y));
    let is_board =
        |at: Point| is_inside(at, &outline) && !holes.iter().any(|hole| is_inside(at, hole));

    // Cut the panel into cells along every corner's lines, keep the cells with board, and
    // join neighbours along u into longer blocks.
    let mut blocks = Vec::new();
    for v in vs.windows(2) {
        let mut runs: Vec<(f64, f64)> = Vec::new();
        for u in us.windows(2) {
            if !is_board(Point::new((u[0] + u[1]) / 2.0, (v[0] + v[1]) / 2.0)) {
                continue;
            }
            match runs.last_mut() {
                Some(run) if (run.1 - u[0]).abs() <= TOUCH_MM => run.1 = u[1],
                _ => runs.push((u[0], u[1])),
            }
        }
        blocks.extend(runs.into_iter().map(|(u0, u1)| {
            let mut block = Block {
                min: [0.0; 3],
                max: [0.0; 3],
            };
            block.min[normal] = plane - thickness_mm / 2.0;
            block.max[normal] = plane + thickness_mm / 2.0;
            (block.min[u_axis], block.max[u_axis]) = (u0, u1);
            (block.min[v_axis], block.max[v_axis]) = (v[0], v[1]);
            block
        }));
    }
    Ok((normal, blocks))
}

/// The board of every folded panel, in panel order.
///
/// Holes with a curved or slanted edge (product holes) count as board: that can only add
/// collisions, never hide one. Boxes have none.
pub fn solids((panels, thickness_mm): (&[FoldedPanel], f64)) -> Result<Vec<Solid>, String> {
    panels
        .iter()
        .enumerate()
        .map(|(index, panel)| solid((index, panel, thickness_mm)))
        .collect()
}

/// Pairs of panels whose board runs into each other.
///
/// Two panels joined by a quarter-turn hinge are left out: in the middle-plane model
/// their boards overlap in a square half a board across, along the crease. Board that
/// stands square to board can only overlap there, so nothing else is missed.
pub fn collisions((solids, hinges): (&[Solid], &[Hinge])) -> Vec<(usize, usize)> {
    let joined = |pair: (usize, usize)| {
        hinges
            .iter()
            .any(|hinge| pair == (hinge.parent, hinge.child) || pair == (hinge.child, hinge.parent))
    };
    let mut pairs = Vec::new();
    for (i, first) in solids.iter().enumerate() {
        for (j, second) in solids.iter().enumerate().skip(i + 1) {
            if joined((i, j)) && first.normal != second.normal {
                continue;
            }
            let hit = first
                .blocks
                .iter()
                .any(|a| second.blocks.iter().any(|b| a.collides(b)));
            if hit {
                pairs.push((i, j));
            }
        }
    }
    pairs
}

/// Hinges whose child no longer touches its parent once folded: the board has come apart
/// at the crease.
pub fn loose_hinges((solids, hinges): (&[Solid], &[Hinge])) -> Vec<(usize, usize)> {
    let touch = |(a, b): (&Solid, &Solid)| {
        a.blocks.iter().any(|first| {
            b.blocks
                .iter()
                .any(|second| first.overlap(second).iter().all(|&o| o >= -TOUCH_MM))
        })
    };
    hinges
        .iter()
        .filter(|hinge| !touch((&solids[hinge.parent], &solids[hinge.child])))
        .map(|hinge| (hinge.parent, hinge.child))
        .collect()
}

/// The block round all the board.
pub fn bounds(solids: &[Solid]) -> Option<Block> {
    solids
        .iter()
        .flat_map(|solid| &solid.blocks)
        .copied()
        .reduce(|all, block| Block {
            min: [0, 1, 2].map(|axis| all.min[axis].min(block.min[axis])),
            max: [0, 1, 2].map(|axis| all.max[axis].max(block.max[axis])),
        })
}
