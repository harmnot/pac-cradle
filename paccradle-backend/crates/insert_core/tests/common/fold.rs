//! Folds a generated piece along its hinges (CLAUDE.md §7.2), the way the 3D preview
//! will, so tests can check what the flat drawing turns into. Panels turn about their
//! creases on the board's middle plane (see `insert_core::piece`). A panel whose hinge has
//! an offset keeps a bend next to its crease in its parent's plane, and turns past it.

use insert_core::{
    Area, FoldDirection, Hinge, Mount, Panel, Piece, Point, Polygon, Region, Stance,
};

use super::measure::distance_to_outline;

/// How far a hinge's ends may sit from its panels' edges: the polygon library snaps to
/// 0.1 µm.
const ON_EDGE_MM: f64 = 1e-3;

/// A point in the insert's frame: x and y as in the drawing, z up from the box floor.
pub type V3 = [f64; 3];

/// How far a band of the drawing runs along a crease, both ways: far longer than any piece.
const BAND_REACH_MM: f64 = 10_000.0;

/// One panel, fully folded, on the board's middle plane.
pub struct FoldedPanel {
    /// The part that turned: the whole panel unless its hinge has an offset.
    pub outline: Vec<V3>,
    pub holes: Vec<Vec<V3>>,
    /// The bend, when the panel's hinge has an offset: the strip next to the crease, which
    /// stays in the parent's plane.
    pub bend: Option<Vec<V3>>,
}

/// A panel cut where it turns: the bend before that line, if any, and the rest.
struct Split {
    turning: Panel,
    bend: Option<Polygon>,
}

/// No turn.
const IDENTITY: [[f64; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

/// A turn followed by a shift: `rot · p + shift`.
#[derive(Debug, Clone, Copy)]
struct Pose {
    rot: [[f64; 3]; 3],
    shift: V3,
}

impl Pose {
    fn apply(&self, p: V3) -> V3 {
        let r = &self.rot;
        [0, 1, 2].map(|i| r[i][0] * p[0] + r[i][1] * p[1] + r[i][2] * p[2] + self.shift[i])
    }

    /// `inner` first, then this pose.
    fn after(&self, inner: &Pose) -> Pose {
        let (a, b) = (&self.rot, &inner.rot);
        let rot = [0, 1, 2].map(|i| [0, 1, 2].map(|j| (0..3).map(|k| a[i][k] * b[k][j]).sum()));
        let turned = self.apply(inner.shift);
        Pose { rot, shift: turned }
    }

    /// A turn of `angle_deg` about the line from `from` to `to`, in the flat plane z = 0.
    fn about((from, to, angle_deg): (Point, Point, f64)) -> Pose {
        let length = from.distance_to(to);
        let d = [(to.x - from.x) / length, (to.y - from.y) / length, 0.0];
        let (sin, cos) = angle_deg.to_radians().sin_cos();
        // Rodrigues: R = cos·I + sin·[d]× + (1 − cos)·d·dᵀ.
        let cross = [[0.0, -d[2], d[1]], [d[2], 0.0, -d[0]], [-d[1], d[0], 0.0]];
        let rot = [0, 1, 2].map(|i| {
            [0, 1, 2].map(|j| {
                let identity = if i == j { 1.0 } else { 0.0 };
                cos * identity + sin * cross[i][j] + (1.0 - cos) * d[i] * d[j]
            })
        });
        let a = [from.x, from.y, 0.0];
        let turned = Pose {
            rot,
            shift: [0.0; 3],
        }
        .apply(a);
        Pose {
            rot,
            shift: [0, 1, 2].map(|i| a[i] - turned[i]),
        }
    }
}

fn flat(point: Point) -> V3 {
    [point.x, point.y, 0.0]
}

fn centroid(polygon: &Polygon) -> V3 {
    let count = polygon.points().len() as f64;
    let sum = polygon
        .points()
        .iter()
        .fold([0.0; 3], |sum, &p| [sum[0] + p.x, sum[1] + p.y, 0.0]);
    [sum[0] / count, sum[1] / count, 0.0]
}

/// Unit vector in the flat plane, square to the crease, pointing into the child.
fn into_child((hinge, child): (&Hinge, &Polygon)) -> [f64; 2] {
    let (from, to) = (hinge.axis.from, hinge.axis.to);
    let length = from.distance_to(to);
    let square = [-(to.y - from.y) / length, (to.x - from.x) / length];
    let center = centroid(child);
    match (center[0] - from.x) * square[0] + (center[1] - from.y) * square[1] >= 0.0 {
        true => square,
        false => square.map(|v| -v),
    }
}

/// The band of the drawing from `near` to `far` mm past the crease, into the child.
fn band((hinge, inward, (near, far)): (&Hinge, [f64; 2], (f64, f64))) -> Polygon {
    let (from, to) = (hinge.axis.from, hinge.axis.to);
    let length = from.distance_to(to);
    let along = [(to.x - from.x) / length, (to.y - from.y) / length];
    let at = |run: f64, out: f64| {
        Point::new(
            from.x + along[0] * run + inward[0] * out,
            from.y + along[1] * run + inward[1] * out,
        )
    };
    let reach = BAND_REACH_MM;
    Polygon(vec![
        at(-reach, near),
        at(length + reach, near),
        at(length + reach, far),
        at(-reach, far),
    ])
}

/// The part of `panel` inside `band`, which must be one region.
fn cut((panel, band): (&Panel, Polygon)) -> Result<Region, String> {
    let board = Area::from_polygon(&panel.outline)
        .and_then(|outline| outline.difference(&Area::from_polygons(&panel.holes)?))
        .and_then(|board| board.intersection(&Area::from_polygon(&band)?))
        .map_err(|error| error.to_string())?;
    let regions = board.into_regions();
    let count = regions.len();
    <[Region; 1]>::try_from(regions)
        .map(|[region]| region)
        .map_err(|_| format!("a bend cut a panel into {count} parts"))
}

/// The child cut at the line `offset_mm` past its crease, where it turns.
fn split((hinge, child): (&Hinge, &Panel)) -> Result<Split, String> {
    if hinge.offset_mm <= 0.0 {
        return Ok(Split {
            turning: child.clone(),
            bend: None,
        });
    }
    let inward = into_child((hinge, &child.outline));
    let offset = hinge.offset_mm;
    let turning = cut((child, band((hinge, inward, (offset, BAND_REACH_MM)))))?;
    let bend = cut((child, band((hinge, inward, (0.0, offset)))))?;
    Ok(Split {
        turning: Panel {
            outline: turning.outer,
            holes: turning.holes,
        },
        bend: Some(bend.outer),
    })
}

/// The child's pose in its parent's flat frame: turned about the line `offset_mm` past
/// the crease (the crease itself without an offset), towards −z for a mountain fold (away
/// from the drawing's face), towards +z for a valley fold.
fn hinge_pose((hinge, inward, turning): (&Hinge, [f64; 2], &Polygon)) -> Pose {
    let wanted = match hinge.direction {
        FoldDirection::Mountain => -1.0,
        FoldDirection::Valley => 1.0,
    };
    let past = |p: Point| {
        Point::new(
            p.x + inward[0] * hinge.offset_mm,
            p.y + inward[1] * hinge.offset_mm,
        )
    };
    let (from, to) = (past(hinge.axis.from), past(hinge.axis.to));
    let turn = |sign: f64| Pose::about((from, to, sign * hinge.fold_deg));
    let forward = turn(1.0);
    match forward.apply(centroid(turning))[2] * wanted >= 0.0 {
        true => forward,
        false => turn(-1.0),
    }
}

/// Where the root panel goes: the drawing turned to the mount's stance, its middle plane
/// on `at_mm` (a flat mount's `at_mm` is on its underside). The drawing's face, its +z,
/// ends up on the side the `Stance` docs name.
fn root_pose((mount, thickness_mm): (&Mount, f64)) -> Pose {
    let [x, y, z] = mount.at_mm;
    // The columns are where the drawing's x, y and face end up.
    let (rot, lift) = match mount.stance {
        Stance::Flat => (IDENTITY, thickness_mm / 2.0),
        // (u, v) → (u, 0, −v), face along +y.
        Stance::UprightAlongX => ([[1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, -1.0, 0.0]], 0.0),
        // (u, v) → (0, u, −v), face along −x.
        Stance::UprightAlongY => ([[0.0, 0.0, -1.0], [1.0, 0.0, 0.0], [0.0, -1.0, 0.0]], 0.0),
    };
    Pose {
        rot,
        shift: [x, y, z + lift],
    }
}

/// What's wrong with the hinges of a flat piece: every hinge needs its crease line, and
/// both ends on the edges of both its panels.
pub fn hinge_problems(piece: &Piece) -> Vec<String> {
    let mut problems = Vec::new();
    if piece.creases.len() != piece.hinges.len() {
        problems.push("every hinge needs its crease line".to_owned());
    }
    for (index, hinge) in piece.hinges.iter().enumerate() {
        for end in [hinge.axis.from, hinge.axis.to] {
            for panel in [hinge.parent, hinge.child] {
                if distance_to_outline(end, &piece.panels[panel].outline) > ON_EDGE_MM {
                    problems.push(format!("hinge {index} isn't on the edge of panel {panel}"));
                }
            }
        }
    }
    problems
}

/// Folds every panel of `piece` fully, placed by `mount`. Fails when the hinges don't
/// join the panels into one tree rooted at panel 0.
pub fn fold(
    (piece, mount, thickness_mm): (&Piece, &Mount, f64),
) -> Result<Vec<FoldedPanel>, String> {
    let mut poses: Vec<Option<Pose>> = vec![None; piece.panels.len()];
    poses[0] = Some(root_pose((mount, thickness_mm)));
    let mut parents = vec![0_usize; piece.panels.len()];
    for hinge in &piece.hinges {
        let Some(count) = parents.get_mut(hinge.child) else {
            return Err(format!("hinge to missing panel {}", hinge.child));
        };
        *count += 1;
    }
    if parents[0] > 0 || parents.iter().skip(1).any(|&count| count != 1) {
        return Err(format!(
            "every panel but the root needs exactly one parent: {parents:?}"
        ));
    }
    let mut splits: Vec<Split> = piece
        .panels
        .iter()
        .map(|panel| Split {
            turning: panel.clone(),
            bend: None,
        })
        .collect();
    for hinge in &piece.hinges {
        splits[hinge.child] = split((hinge, &piece.panels[hinge.child]))?;
    }
    loop {
        let mut progressed = false;
        for hinge in &piece.hinges {
            let (Some(parent), None) = (poses[hinge.parent], poses[hinge.child]) else {
                continue;
            };
            let inward = into_child((hinge, &piece.panels[hinge.child].outline));
            let local = hinge_pose((hinge, inward, &splits[hinge.child].turning.outline));
            poses[hinge.child] = Some(parent.after(&local));
            progressed = true;
        }
        if !progressed {
            break;
        }
    }
    let mut parent_of: Vec<Option<usize>> = vec![None; piece.panels.len()];
    for hinge in &piece.hinges {
        parent_of[hinge.child] = Some(hinge.parent);
    }
    let place = |(ring, pose): (&Polygon, Pose)| -> Vec<V3> {
        ring.points().iter().map(|&p| pose.apply(flat(p))).collect()
    };
    splits
        .iter()
        .enumerate()
        .map(|(index, split)| {
            let pose = poses[index].ok_or(format!("panel {index} isn't joined to the root"))?;
            // The bend stays with the parent.
            let bend_pose = parent_of[index].and_then(|parent| poses[parent]);
            Ok(FoldedPanel {
                outline: place((&split.turning.outline, pose)),
                holes: split
                    .turning
                    .holes
                    .iter()
                    .map(|hole| place((hole, pose)))
                    .collect(),
                bend: split
                    .bend
                    .as_ref()
                    .zip(bend_pose)
                    .map(|(bend, pose)| place((bend, pose))),
            })
        })
        .collect()
}
