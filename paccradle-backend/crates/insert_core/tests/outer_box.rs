//! M1-A6: the outer boxes. The main check folds the generated piece along its hinges and
//! looks at the result, not at the generator's own numbers. It checks that:
//! - no panel runs into another;
//! - nothing reaches into the inside;
//! - each side of the inside is at least 90% closed, with board touching it;
//! - the folded box measures the outside size the generator reports;
//! - the mailer's lock tabs fill their slots and end flush with the base underside.
//!
//! Also: the sizes round the insert, fixed boxes and the errors.
// The helpers below are test code too; clippy.toml only exempts `#[test]` functions.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::fold::{V3, fold, hinge_problems};
use common::measure::is_inside;
use common::solid::{Block, Solid, TOUCH_MM, bounds, collisions, in_plane, loose_hinges, solids};
use insert_core::{
    Align, Area, Board, BoxSize, BoxSpec, BoxStyle, CutoutCardSpec, Design, FoldDirection,
    GenerateError, InsertSpec, Material, OuterBox, Packaging, Piece, Point, Polygon, Transform,
};

/// Least share of each side of the inside that board must close.
const MIN_CLOSED: f64 = 0.9;

/// The mailer's fold stages, in the order the box goes together.
const MAILER_WALLS: u32 = 0;
const DUST_FLAPS: u32 = 1;
const OUTER_ENDS: u32 = 2;
const RIMS: u32 = 3;
const INNER_ENDS: u32 = 4;
const LID: u32 = 5;
const TUCK: u32 = 6;

/// The shipping box's fold stages.
const SHIPPING_WALLS: u32 = 0;
const BOTTOM_MINOR: u32 = 1;
const BOTTOM_MAJOR: u32 = 2;
const TOP_MINOR: u32 = 3;
const TOP_MAJOR: u32 = 4;

fn generate(design: &Design) -> Packaging {
    match design.generate() {
        Ok(packaging) => packaging,
        Err(error) => panic!("generate: {error}"),
    }
}

fn outer_box(packaging: &Packaging) -> &OuterBox {
    packaging.outer_box.as_ref().expect("the design has a box")
}

fn box_spec((style, board): (BoxStyle, Board)) -> Option<BoxSpec> {
    Some(BoxSpec {
        style,
        material: Material::starting_values(board),
        inside_mm: None,
    })
}

fn fixed((style, board): (BoxStyle, Board), inside_mm: [f64; 3]) -> Option<BoxSpec> {
    let [length_mm, width_mm, height_mm] = inside_mm;
    Some(BoxSpec {
        inside_mm: Some(BoxSize {
            length_mm,
            width_mm,
            height_mm,
        }),
        ..box_spec((style, board)).unwrap()
    })
}

/// The lip gloss and compact card (both 95 mm tall) in a box of the given kind.
fn card_in(outer_box: Option<BoxSpec>) -> Design {
    Design {
        outer_box,
        ..common::card_design()
    }
}

/// Six candle jars in a partition grid, in a box of the given kind.
fn grid_in(outer_box: Option<BoxSpec>) -> Design {
    Design {
        outer_box,
        ..common::grid_design()
    }
}

/// A 10 × 10 × 5 mm part on a paperboard card with a 3 mm margin: smaller than any box.
fn tiny_in(outer_box: Option<BoxSpec>) -> Design {
    let mut part = common::product("part", common::rectangle(10.0, 10.0));
    part.height_mm = 5.0;
    Design {
        material: Material::starting_values(Board::Paperboard),
        products: vec![part],
        insert: InsertSpec::CutoutCard(CutoutCardSpec {
            margin_mm: 3.0,
            finger_notches: None,
            align: Align::Center,
            placements: Vec::new(),
        }),
        outer_box,
    }
}

/// The panels that fold at `stage`, in hinge order.
fn panels_at((piece, stage): (&Piece, u32)) -> Vec<usize> {
    piece
        .hinges
        .iter()
        .filter(|hinge| hinge.stage == stage)
        .map(|hinge| hinge.child)
        .collect()
}

/// Moves the far edge of a hinged panel (its points farthest from the crease) `by_mm`
/// further out, making the panel longer, or shorter when negative.
fn lengthen((piece, child, by_mm): (&mut Piece, usize, f64)) {
    let hinge = *piece
        .hinges
        .iter()
        .find(|hinge| hinge.child == child)
        .unwrap();
    let (from, to) = (hinge.axis.from, hinge.axis.to);
    let length = from.distance_to(to);
    // Unit normal to the crease, in the drawing.
    let normal = Point::new(-(to.y - from.y) / length, (to.x - from.x) / length);
    let side = |p: Point| (p.x - from.x) * normal.x + (p.y - from.y) * normal.y;
    let outline = &mut piece.panels[child].outline;
    let far = outline
        .points()
        .iter()
        .map(|&p| side(p).abs())
        .fold(0.0, f64::max);
    *outline = Polygon(
        outline
            .points()
            .iter()
            .map(|&p| match (side(p).abs() - far).abs() <= TOUCH_MM {
                true => {
                    let push = by_mm * side(p).signum();
                    Point::new(p.x + push * normal.x, p.y + push * normal.y)
                }
                false => p,
            })
            .collect(),
    );
}

/// How deep a hinged panel reaches from its crease.
fn depth((piece, child): (&Piece, usize)) -> f64 {
    let hinge = piece
        .hinges
        .iter()
        .find(|hinge| hinge.child == child)
        .unwrap();
    piece.panels[child]
        .outline
        .points()
        .iter()
        .map(|&p| hinge.axis.distance_to(p))
        .fold(0.0, f64::max)
}

/// One side of the inside: the axis it looks along and which end of it.
#[derive(Debug, Clone, Copy)]
struct Side {
    name: &'static str,
    axis: usize,
    far_end: bool,
}

const SIDES: [Side; 6] = [
    Side {
        name: "bottom",
        axis: 2,
        far_end: false,
    },
    Side {
        name: "top",
        axis: 2,
        far_end: true,
    },
    Side {
        name: "back",
        axis: 1,
        far_end: false,
    },
    Side {
        name: "front",
        axis: 1,
        far_end: true,
    },
    Side {
        name: "left end",
        axis: 0,
        far_end: false,
    },
    Side {
        name: "right end",
        axis: 0,
        far_end: true,
    },
];

/// The inside, in the insert's frame: centred on the insert, floor at z = 0.
fn inside_block((outer_box, packaging): (&OuterBox, &Packaging)) -> Block {
    let inside = outer_box.inside_mm;
    let insert = &packaging.insert;
    let min = [
        (insert.length_mm - inside.length_mm) / 2.0,
        (insert.width_mm - inside.width_mm) / 2.0,
        0.0,
    ];
    Block {
        min,
        max: [
            min[0] + inside.length_mm,
            min[1] + inside.width_mm,
            inside.height_mm,
        ],
    }
}

/// How far out the nearest board beyond a side of the inside stands, and how much of the
/// side that board closes (0–1). `None` when no board covers the side at all.
fn closing((side, room, solids): (Side, &Block, &[Solid])) -> Option<(f64, f64)> {
    let [u, v] = in_plane(side.axis);
    let plane = match side.far_end {
        true => room.max[side.axis],
        false => room.min[side.axis],
    };
    // How far a block stands out from the side, when it lies wholly beyond it.
    let out = |block: &Block| match side.far_end {
        true => block.min[side.axis] - plane,
        false => plane - block.max[side.axis],
    };
    let face = Block {
        min: room.min,
        max: room.max,
    };
    let mut nearest: Option<f64> = None;
    let mut cover = Vec::new();
    for block in solids.iter().flat_map(|solid| &solid.blocks) {
        let overlap = block.overlap(&face);
        if out(block) < -TOUCH_MM || overlap[u] <= TOUCH_MM || overlap[v] <= TOUCH_MM {
            continue;
        }
        nearest = Some(nearest.map_or(out(block), |near: f64| near.min(out(block))));
        let corner = |at: &V3| Point::new(at[u], at[v]);
        let (low, high) = (
            corner(&[0, 1, 2].map(|axis| block.min[axis].max(room.min[axis]))),
            corner(&[0, 1, 2].map(|axis| block.max[axis].min(room.max[axis]))),
        );
        cover.push(Polygon(vec![
            low,
            Point::new(high.x, low.y),
            high,
            Point::new(low.x, high.y),
        ]));
    }
    let closed = Area::from_polygons(&cover).unwrap().area_mm2();
    let whole = (room.max[u] - room.min[u]) * (room.max[v] - room.min[v]);
    nearest.map(|gap| (gap, closed / whole))
}

/// The mailer's lock tabs, read from the folded box: board in a base slot ends flush with
/// the base's underside, and every slot gets some. Board running into the base around a
/// slot is a collision, which [`box_problems`] reports.
fn tab_problems((solids, piece, base_holes): (&[Solid], &Piece, &[Polygon])) -> Vec<String> {
    let Some(base) = solids[0].blocks.first() else {
        return vec!["the base has no board".to_owned()];
    };
    let (underside, top) = (base.min[2], base.max[2]);
    // The walls hinged to the base dip half a board into it along their creases.
    let hinged_to_base = |index: &usize| {
        piece
            .hinges
            .iter()
            .any(|hinge| hinge.parent == 0 && hinge.child == *index)
    };
    let tabs: Vec<&Block> = (1..solids.len())
        .filter(|index| !hinged_to_base(index))
        .flat_map(|index| &solids[index].blocks)
        .filter(|block| block.min[2] < top - TOUCH_MM && block.max[2] > underside + TOUCH_MM)
        .collect();
    let mut problems: Vec<String> = tabs
        .iter()
        .filter(|tab| (tab.min[2] - underside).abs() > TOUCH_MM)
        .map(|tab| {
            format!(
                "a tab ends {:.3} mm from the base underside",
                tab.min[2] - underside
            )
        })
        .collect();
    let middle = |tab: &&Block| {
        Point::new(
            (tab.min[0] + tab.max[0]) / 2.0,
            (tab.min[1] + tab.max[1]) / 2.0,
        )
    };
    for slot in base_holes {
        if !tabs.iter().any(|tab| is_inside(middle(tab), slot)) {
            problems.push("a lock slot gets no tab".to_owned());
        }
    }
    problems
}

/// What's wrong with the folded box: an empty list when it folds up right.
fn box_problems((packaging, design): (&Packaging, &Design)) -> Vec<String> {
    let outer_box = outer_box(packaging);
    let thickness = design.outer_box.unwrap().material.thickness_mm;
    if outer_box.pieces.len() != 1 || outer_box.mounts.len() != 1 {
        return vec!["a box is one piece, mounted once".to_owned()];
    }
    let piece = &outer_box.pieces[0];
    let mut problems = hinge_problems(piece);
    let folded = match fold((piece, &outer_box.mounts[0], thickness)) {
        Ok(folded) => folded,
        Err(error) => return [problems, vec![error]].concat(),
    };
    let solids = match solids((&folded, thickness)) {
        Ok(solids) => solids,
        Err(error) => return [problems, vec![error]].concat(),
    };
    for (i, j) in collisions((&solids, &piece.hinges)) {
        problems.push(format!("panels {i} and {j} run into each other"));
    }
    for (parent, child) in loose_hinges((&solids, &piece.hinges)) {
        problems.push(format!("panel {child} comes loose from panel {parent}"));
    }

    let room = inside_block((outer_box, packaging));
    for (index, solid) in solids.iter().enumerate() {
        if solid.blocks.iter().any(|block| block.collides(&room)) {
            problems.push(format!("panel {index} reaches into the inside"));
        }
    }
    let tallest = design
        .products
        .iter()
        .map(|product| product.height_mm)
        .fold(packaging.insert.height_mm, f64::max);
    let insert = &packaging.insert;
    let fits = room.min[0] <= TOUCH_MM
        && room.min[1] <= TOUCH_MM
        && room.max[0] >= insert.length_mm - TOUCH_MM
        && room.max[1] >= insert.width_mm - TOUCH_MM
        && room.max[2] >= tallest - TOUCH_MM;
    if !fits {
        problems.push("the insert or its products don't fit the inside".to_owned());
    }
    for side in SIDES {
        let Some((gap, closed)) = closing((side, &room, &solids)) else {
            problems.push(format!("nothing closes the {}", side.name));
            continue;
        };
        if gap > TOUCH_MM {
            problems.push(format!("the {} stands {gap:.3} mm out", side.name));
        }
        if closed < MIN_CLOSED {
            problems.push(format!(
                "the {} is only {:.1}% closed",
                side.name,
                closed * 100.0
            ));
        }
    }

    let outside = outer_box.outside_mm;
    let reported = [outside.length_mm, outside.width_mm, outside.height_mm];
    let folded_size =
        bounds(&solids).map(|all| [0, 1, 2].map(|axis| all.max[axis] - all.min[axis]));
    let matches = folded_size.is_some_and(|size| {
        size.iter()
            .zip(reported)
            .all(|(folded, reported)| (folded - reported).abs() <= TOUCH_MM)
    });
    if !matches {
        problems.push(format!(
            "folded, the box measures {folded_size:?}, not the reported {reported:?}"
        ));
    }

    if outer_box.style == BoxStyle::Mailer {
        let base_holes: Vec<Polygon> = folded[0]
            .holes
            .iter()
            .map(|ring| Polygon(ring.iter().map(|p| Point::new(p[0], p[1])).collect()))
            .collect();
        problems.extend(tab_problems((&solids, piece, &base_holes)));
    }
    problems
}

fn assert_folds_up(design: &Design) -> Packaging {
    let packaging = generate(design);
    let problems = box_problems((&packaging, design));
    assert!(problems.is_empty(), "{problems:#?}");
    packaging
}

/// A named way to break a generated box on purpose, and part of the problem the check
/// must report for it.
type Break = (&'static str, fn(&mut OuterBox), &'static str);

fn assert_breaks_are_caught((design, breaks): (&Design, &[Break])) {
    let packaging = assert_folds_up(design);
    for &(name, break_it, expected) in breaks {
        let mut broken = packaging.clone();
        break_it(broken.outer_box.as_mut().unwrap());
        let problems = box_problems((&broken, design));
        assert!(
            problems.iter().any(|problem| problem.contains(expected)),
            "{name} wasn't caught as `{expected}`: {problems:#?}"
        );
    }
}

fn piece(outer_box: &mut OuterBox) -> &mut Piece {
    &mut outer_box.pieces[0]
}

#[test]
fn generate_returns_the_insert_and_its_box() {
    let design = common::cradle_design();
    let packaging = generate(&design);
    assert_eq!(Ok(packaging.insert.clone()), design.generate_insert());
    assert_eq!(outer_box(&packaging).style, BoxStyle::Mailer);

    let no_box = generate(&common::grid_design());
    assert_eq!(no_box.outer_box, None);
}

#[test]
fn mailer_is_one_piece_of_15_panels() {
    let packaging = generate(&common::cradle_design());
    let outer_box = outer_box(&packaging);
    assert_eq!(outer_box.pieces.len(), 1);
    let piece = &outer_box.pieces[0];
    assert_eq!((piece.name.as_str(), piece.quantity), ("mailer", 1));
    assert_eq!(piece.panels.len(), 15);
    let tree: Vec<(usize, usize, u32)> = piece
        .hinges
        .iter()
        .map(|hinge| (hinge.parent, hinge.child, hinge.stage))
        .collect();
    assert_eq!(
        tree,
        [
            (0, 1, MAILER_WALLS),
            (1, 2, LID),
            (2, 3, TUCK),
            (1, 4, DUST_FLAPS),
            (1, 5, DUST_FLAPS),
            (0, 6, MAILER_WALLS),
            (6, 7, DUST_FLAPS),
            (6, 8, DUST_FLAPS),
            (0, 9, OUTER_ENDS),
            (9, 10, RIMS),
            (10, 11, INNER_ENDS),
            (0, 12, OUTER_ENDS),
            (12, 13, RIMS),
            (13, 14, INNER_ENDS),
        ]
    );
    for hinge in &piece.hinges {
        assert_eq!(hinge.fold_deg, 90.0);
        assert_eq!(hinge.direction, FoldDirection::Valley);
        assert_eq!(hinge.offset_mm, 0.0);
    }
    // Two lock tabs on each inner end wall, through two slots each end of the base.
    assert_eq!(piece.panels[0].holes.len(), 4);
}

#[test]
fn mailer_folds_into_a_closed_box() {
    let fixture: Design =
        serde_json::from_str(include_str!("fixtures/cradle_mailer.json")).unwrap();
    let mailer = |board| box_spec((BoxStyle::Mailer, board));
    let designs = [
        common::cradle_design(),
        fixture,
        grid_in(mailer(Board::BFlute)),
        card_in(mailer(Board::Paperboard)),
        card_in(fixed(
            (BoxStyle::Mailer, Board::CFlute),
            [200.0, 150.0, 100.0],
        )),
        // A long box, so each inner end wall gets more than two tabs.
        card_in(fixed(
            (BoxStyle::Mailer, Board::EFlute),
            [160.0, 240.0, 100.0],
        )),
    ];
    for design in &designs {
        assert_folds_up(design);
    }
    let long = generate(&designs[5]);
    assert!(outer_box(&long).pieces[0].panels[0].holes.len() > 4);
}

#[test]
fn shipping_box_is_one_piece_of_13_panels() {
    let design = card_in(fixed(
        (BoxStyle::Shipping, Board::BFlute),
        [160.0, 120.0, 100.0],
    ));
    let packaging = generate(&design);
    let piece = &outer_box(&packaging).pieces[0];
    assert_eq!((piece.name.as_str(), piece.quantity), ("shipping box", 1));
    assert_eq!(piece.panels.len(), 13);
    let tree: Vec<(usize, usize, u32)> = piece
        .hinges
        .iter()
        .map(|hinge| (hinge.parent, hinge.child, hinge.stage))
        .collect();
    // Back and front run along x, the longer side, so their flaps are the major ones.
    assert_eq!(
        tree,
        [
            (0, 1, SHIPPING_WALLS),
            (1, 2, SHIPPING_WALLS),
            (2, 3, SHIPPING_WALLS),
            (3, 4, SHIPPING_WALLS),
            (0, 5, BOTTOM_MAJOR),
            (0, 6, TOP_MAJOR),
            (1, 7, BOTTOM_MINOR),
            (1, 8, TOP_MINOR),
            (2, 9, BOTTOM_MAJOR),
            (2, 10, TOP_MAJOR),
            (3, 11, BOTTOM_MINOR),
            (3, 12, TOP_MINOR),
        ]
    );
    for hinge in &piece.hinges {
        assert_eq!(hinge.fold_deg, 90.0);
        assert_eq!(hinge.direction, FoldDirection::Valley);
    }
}

#[test]
fn shipping_box_walls_and_flaps_line_up() {
    // B-flute: 3 mm board.
    let design = card_in(fixed(
        (BoxStyle::Shipping, Board::BFlute),
        [160.0, 120.0, 100.0],
    ));
    let packaging = generate(&design);
    let outer_box = outer_box(&packaging);
    let piece = &outer_box.pieces[0];
    let span = |panel: usize| {
        let bounds = piece.panels[panel].outline.bounds().unwrap();
        (bounds.min.y, bounds.max.y)
    };
    let level = |values: Vec<f64>| {
        values
            .windows(2)
            .all(|pair| (pair[0] - pair[1]).abs() < 1e-9)
    };

    // Flat: one straight score line top and bottom, every wall as tall, every flap as
    // deep, so the flap tips line up.
    let walls = [0, 1, 2, 3];
    assert!(level(walls.map(|wall| span(wall).0).to_vec()), "wall tops");
    assert!(
        level(walls.map(|wall| span(wall).1).to_vec()),
        "wall bottoms"
    );
    let flaps = |stages: [u32; 2]| -> Vec<usize> {
        stages
            .into_iter()
            .flat_map(|stage| panels_at((piece, stage)))
            .collect()
    };
    let (top, bottom) = (
        flaps([TOP_MINOR, TOP_MAJOR]),
        flaps([BOTTOM_MINOR, BOTTOM_MAJOR]),
    );
    assert!(
        level(top.iter().map(|&flap| span(flap).0).collect()),
        "top tips"
    );
    assert!(
        level(bottom.iter().map(|&flap| span(flap).1).collect()),
        "bottom tips"
    );
    let depths: Vec<f64> = top
        .iter()
        .chain(&bottom)
        .map(|&flap| depth((piece, flap)))
        .collect();
    assert!(level(depths), "flap depths");

    // Only the major flaps turn past their crease, a board out, round the minor ones.
    for hinge in &piece.hinges {
        let major = [BOTTOM_MAJOR, TOP_MAJOR].contains(&hinge.stage);
        assert_eq!(hinge.offset_mm, if major { 3.0 } else { 0.0 });
    }

    // Folded: all four walls stand from the same height to the same height.
    let folded = fold((piece, &outer_box.mounts[0], 3.0)).unwrap();
    let heights = |extreme: fn(f64, f64) -> f64| -> Vec<f64> {
        walls
            .iter()
            .map(|&wall| {
                folded[wall]
                    .outline
                    .iter()
                    .map(|p| p[2])
                    .reduce(extreme)
                    .unwrap()
            })
            .collect()
    };
    assert!(level(heights(f64::min)), "folded wall bottoms");
    assert!(level(heights(f64::max)), "folded wall tops");

    // The outer bottom flaps (back 5, front 9) bend a board down from the score, stay
    // joined to their walls, and meet in the middle with the 0.5 mm tolerance between.
    for flap in [5, 9] {
        let bend = folded[flap].bend.as_ref().expect("an outer flap bends");
        let zs = bend.iter().map(|p| p[2]);
        let (low, high) = (
            zs.clone().fold(f64::INFINITY, f64::min),
            zs.fold(f64::NEG_INFINITY, f64::max),
        );
        assert!(
            (low + 4.5).abs() < 1e-6 && (high + 1.5).abs() < 1e-6,
            "bend from {high} to {low}"
        );
    }
    assert!(
        folded[7].bend.is_none() && folded[11].bend.is_none(),
        "inner flaps fold on the score"
    );
    let ys = |flap: usize| folded[flap].outline.iter().map(|p| p[1]);
    let back_tip = ys(5).fold(f64::NEG_INFINITY, f64::max);
    let front_tip = ys(9).fold(f64::INFINITY, f64::min);
    assert!(
        (front_tip - back_tip - 0.5).abs() < 1e-6,
        "outer flaps {} mm apart",
        front_tip - back_tip
    );
}

#[test]
fn shipping_box_folds_with_either_side_longer() {
    let shipping = |board, inside| card_in(fixed((BoxStyle::Shipping, board), inside));
    let designs = [
        shipping(Board::BFlute, [160.0, 120.0, 100.0]),
        shipping(Board::BFlute, [120.0, 200.0, 100.0]),
        shipping(Board::CFlute, [180.0, 180.0, 100.0]),
        shipping(Board::Paperboard, [160.0, 120.0, 96.0]),
        grid_in(box_spec((BoxStyle::Shipping, Board::EFlute))),
    ];
    for design in &designs {
        assert_folds_up(design);
    }
    // With the width longer, the side walls carry the major flaps.
    let wide = generate(&designs[1]);
    let piece = &outer_box(&wide).pieces[0];
    assert_eq!(panels_at((piece, BOTTOM_MAJOR)), [7, 11]);
}

#[test]
fn smallest_boxes_fold_up() {
    for style in [BoxStyle::Mailer, BoxStyle::Shipping] {
        for board in [Board::Paperboard, Board::CFlute] {
            assert_folds_up(&tiny_in(box_spec((style, board))));
        }
    }
}

#[test]
fn box_is_the_insert_plus_clearance_with_headroom() {
    // E-flute: 1.5 mm clearance each side and over the 95 mm products.
    let design = common::cradle_design();
    let packaging = generate(&design);
    let insert = &packaging.insert;
    assert_eq!(
        outer_box(&packaging).inside_mm,
        BoxSize {
            length_mm: insert.length_mm + 3.0,
            width_mm: insert.width_mm + 3.0,
            height_mm: 96.5,
        }
    );

    // A deck taller than the products sets the height.
    let mut low = design.clone();
    low.products[0].height_mm = 30.0;
    let packaging = assert_folds_up(&low);
    assert_eq!(outer_box(&packaging).inside_mm.height_mm, 40.0 + 1.5);
}

#[test]
fn small_inserts_grow_the_box_to_its_smallest_size() {
    // C-flute: 4 mm board, 0.5 mm slot tolerance, 12 mm webs.
    let mailer = generate(&tiny_in(box_spec((BoxStyle::Mailer, Board::CFlute))));
    let insert_length = mailer.insert.length_mm;
    assert_eq!(
        outer_box(&mailer).inside_mm,
        BoxSize {
            length_mm: (insert_length + 4.0).max(12.0),
            // One lock tab: 3 webs, the tolerance and a 5 mm tab.
            width_mm: 3.0 * 12.0 + 0.5 + 5.0,
            // The dust flaps: a board, the tolerance above and below, and a web.
            height_mm: 4.0 + 1.0 + 12.0,
        }
    );

    let shipping = generate(&tiny_in(box_spec((BoxStyle::Shipping, Board::CFlute))));
    let inside = outer_box(&shipping).inside_mm;
    assert_eq!(
        [inside.length_mm, inside.height_mm],
        [(insert_length + 4.0).max(24.0), 12.0]
    );
}

#[test]
fn boxes_stay_in_one_piece_at_odd_sizes() {
    // Measured inserts give sizes like these. Each panel is built on its own, and must
    // still meet its neighbours exactly: a 118.953 mm long E-flute shipping box once fell
    // into 5 parts (M1-A9).
    for step in 0..25 {
        let step = f64::from(step);
        let inside = [
            160.0 + 0.4537 * step,
            120.0 + 1.3211 * step,
            96.0 + 0.071 * step,
        ];
        for style in [BoxStyle::Mailer, BoxStyle::Shipping] {
            generate(&card_in(fixed((style, Board::EFlute), inside)));
        }
    }
}

#[test]
fn fixed_box_keeps_its_size() {
    let inside = [200.0, 150.0, 100.0];
    for style in [BoxStyle::Mailer, BoxStyle::Shipping] {
        let packaging = generate(&card_in(fixed((style, Board::EFlute), inside)));
        let size = outer_box(&packaging).inside_mm;
        assert_eq!([size.length_mm, size.width_mm, size.height_mm], inside);
    }
}

#[test]
fn products_taller_than_a_fixed_box_are_refused() {
    // The products are 95 mm tall.
    let low = card_in(fixed(
        (BoxStyle::Shipping, Board::BFlute),
        [160.0, 120.0, 40.0],
    ));
    assert_eq!(
        low.generate(),
        Err(GenerateError::TooTallForBox {
            needed_height_mm: 95.0,
            room_height_mm: 40.0,
        })
    );

    // A cradle deck taller than its products counts too.
    let mut cradle = common::cradle_design();
    cradle.products[0].height_mm = 30.0;
    cradle.outer_box = fixed((BoxStyle::Mailer, Board::EFlute), [160.0, 120.0, 35.0]);
    assert_eq!(
        cradle.generate(),
        Err(GenerateError::TooTallForBox {
            needed_height_mm: 40.0,
            room_height_mm: 35.0,
        })
    );
}

#[test]
fn fixed_box_below_the_smallest_size_is_refused() {
    // E-flute: 1.6 mm board, 0.5 mm slot tolerance, 3 × 1.6 mm webs (written the way
    // the generator adds them up, so the floats match).
    let web = 3.0 * 1.6;
    let mailer_min = Err(GenerateError::BoxTooSmall {
        min_length_mm: web,
        min_width_mm: 3.0 * web + 0.5 + 5.0,
        min_height_mm: 1.6 + 2.0 * 0.5 + web,
    });
    let low = tiny_in(fixed((BoxStyle::Mailer, Board::EFlute), [100.0, 80.0, 6.0]));
    assert_eq!(low.generate(), mailer_min);
    // A slim 60 × 4 mm part lies along the box, so the card fits a box this narrow.
    let mut narrow = tiny_in(fixed(
        (BoxStyle::Mailer, Board::EFlute),
        [100.0, 19.0, 50.0],
    ));
    narrow.products[0].contour_mm = common::rectangle(60.0, 4.0);
    assert_eq!(narrow.generate(), mailer_min);

    let short = tiny_in(fixed(
        (BoxStyle::Shipping, Board::CFlute),
        [22.0, 40.0, 40.0],
    ));
    assert_eq!(
        short.generate(),
        Err(GenerateError::BoxTooSmall {
            min_length_mm: 24.0,
            min_width_mm: 12.0,
            min_height_mm: 12.0,
        })
    );
}

#[test]
fn fold_check_catches_broken_mailers() {
    let breaks: [Break; 8] = [
        (
            "dust flaps 1 mm too deep",
            |outer_box| {
                let flap = panels_at((piece(outer_box), DUST_FLAPS))[0];
                lengthen((piece(outer_box), flap, 1.0));
            },
            "run into each other",
        ),
        (
            "slots 0.5 mm off",
            |outer_box| {
                for hole in &mut piece(outer_box).panels[0].holes {
                    *hole = hole.transformed(Transform::shift(Point::new(0.0, 0.5)));
                }
            },
            "run into each other",
        ),
        (
            "tabs 0.5 mm short",
            |outer_box| {
                let inner = panels_at((piece(outer_box), INNER_ENDS))[0];
                lengthen((piece(outer_box), inner, -0.5));
            },
            "from the base underside",
        ),
        (
            "a rim 1 mm too wide",
            |outer_box| {
                let rim = panels_at((piece(outer_box), RIMS))[1];
                lengthen((piece(outer_box), rim, 1.0));
            },
            "reaches into the inside",
        ),
        (
            "an inner end wall folded out",
            |outer_box| {
                let piece = piece(outer_box);
                let hinge = piece
                    .hinges
                    .iter_mut()
                    .find(|h| h.stage == INNER_ENDS)
                    .unwrap();
                hinge.direction = FoldDirection::Mountain;
            },
            "run into each other",
        ),
        (
            "the lid folded back",
            |outer_box| {
                let piece = piece(outer_box);
                let hinge = piece.hinges.iter_mut().find(|h| h.stage == LID).unwrap();
                hinge.direction = FoldDirection::Mountain;
            },
            "nothing closes the top",
        ),
        (
            "the box 1 mm off the insert",
            |outer_box| outer_box.mounts[0].at_mm[1] += 1.0,
            "reaches into the inside",
        ),
        (
            "the wrong outside size",
            |outer_box| outer_box.outside_mm.length_mm += 1.0,
            "not the reported",
        ),
    ];
    assert_breaks_are_caught((&common::cradle_design(), &breaks));
}

#[test]
fn fold_check_catches_broken_shipping_boxes() {
    let breaks: [Break; 9] = [
        (
            "major flaps 1 mm too deep",
            |outer_box| {
                let flap = panels_at((piece(outer_box), BOTTOM_MAJOR))[0];
                lengthen((piece(outer_box), flap, 1.0));
            },
            "run into each other",
        ),
        (
            "the top flaps cut to 20 mm",
            |outer_box| {
                let piece = piece(outer_box);
                let top = [TOP_MINOR, TOP_MAJOR].map(|stage| panels_at((piece, stage)));
                for flap in top.into_iter().flatten() {
                    let by = 20.0 - depth((piece, flap));
                    lengthen((piece, flap, by));
                }
            },
            "the top is only",
        ),
        (
            "major flaps folded on their crease",
            |outer_box| {
                for hinge in &mut piece(outer_box).hinges {
                    hinge.offset_mm = 0.0;
                }
            },
            "run into each other",
        ),
        (
            "an outer flap pulled 1 mm off its wall",
            |outer_box| {
                let piece = piece(outer_box);
                let flap = panels_at((piece, BOTTOM_MAJOR))[0];
                let hinge = *piece.hinges.iter().find(|h| h.child == flap).unwrap();
                // Away from the wall: the bottom flaps hang below the score in the drawing.
                let down = Transform::shift(Point::new(0.0, 1.0));
                piece.panels[flap].outline = piece.panels[flap].outline.transformed(down);
                assert!(hinge.axis.from.y < piece.panels[flap].outline.points()[0].y);
            },
            "comes loose",
        ),
        (
            "a minor flap folded to 80°",
            |outer_box| {
                let piece = piece(outer_box);
                let hinge = piece
                    .hinges
                    .iter_mut()
                    .find(|h| h.stage == BOTTOM_MINOR)
                    .unwrap();
                hinge.fold_deg = 80.0;
            },
            "isn't square to an axis",
        ),
        (
            "a wall folded out",
            |outer_box| piece(outer_box).hinges[0].direction = FoldDirection::Mountain,
            "not the reported",
        ),
        (
            "a crease off its edge",
            |outer_box| piece(outer_box).hinges[1].axis.from.x += 1.0,
            "isn't on the edge",
        ),
        (
            "the box 1 mm off the insert",
            |outer_box| outer_box.mounts[0].at_mm[0] += 1.0,
            "reaches into the inside",
        ),
        (
            "the wrong outside size",
            |outer_box| outer_box.outside_mm.height_mm += 1.0,
            "not the reported",
        ),
    ];
    let design = card_in(fixed(
        (BoxStyle::Shipping, Board::BFlute),
        [160.0, 120.0, 100.0],
    ));
    assert_breaks_are_caught((&design, &breaks));
}
