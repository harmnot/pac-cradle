//! M1-A7: manufacturability checks (CLAUDE.md §6.5). Every rule fires on a design built
//! to break it, with the right numbers, piece, spot and product copies, and the designs
//! the generators make by default raise nothing.
// The helpers below are test code too; clippy.toml only exempts `#[test]` functions.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use insert_core::{
    Align, Board, BoxSize, BoxSpec, BoxStyle, CURVE_TOLERANCE_MM, CheckOptions, CradlePlatformSpec,
    CutoutCardSpec, Design, InsertSpec, Line, Material, Packaging, Part, Path, Placement, Point,
    Polygon, Product, Rule, Segment, SheetSize, SmallFeature, Warning,
};

fn generate(design: &Design) -> Packaging {
    match design.generate() {
        Ok(packaging) => packaging,
        Err(error) => panic!("generate: {error}"),
    }
}

fn checked((packaging, design, options): (&Packaging, &Design, &CheckOptions)) -> Vec<Warning> {
    packaging.warnings((design, options)).unwrap()
}

fn warnings(design: &Design) -> Vec<Warning> {
    checked((&generate(design), design, &CheckOptions::default()))
}

/// The warnings `keep` picks out.
fn only(warnings: &[Warning], keep: fn(&Rule) -> bool) -> Vec<Warning> {
    warnings.iter().filter(|w| keep(&w.rule)).cloned().collect()
}

fn listed(warnings: &[Warning]) -> Vec<String> {
    warnings
        .iter()
        .map(|w| format!("{:?} piece {:?}: {}", w.at.part, w.at.piece, w.rule))
        .collect()
}

fn product((id, contour_mm): (&str, Polygon), quantity: u32) -> Product {
    Product {
        id: id.to_owned(),
        contour_mm,
        height_mm: 50.0,
        quantity,
    }
}

fn placed((id, x, y): (&str, f64, f64)) -> Placement {
    Placement {
        product_id: id.to_owned(),
        center_mm: Point::new(x, y),
        rotation_deg: 0.0,
    }
}

/// A paperboard card (3 mm webs) that grows round its holes, 10 mm margin, no notches.
fn card((products, placements): (Vec<Product>, Vec<Placement>)) -> Design {
    Design {
        material: Material::starting_values(Board::Paperboard),
        products,
        insert: InsertSpec::CutoutCard(CutoutCardSpec {
            margin_mm: 10.0,
            finger_notches: None,
            align: Align::Center,
            placements,
        }),
        outer_box: None,
    }
}

/// A chipboard cradle, deck 40 mm high, in a fixed E-flute mailer: the deck is 157 × 97 mm.
fn cradle(placements: Vec<Placement>) -> Design {
    Design {
        material: Material::starting_values(Board::Chipboard),
        products: vec![product(("perfume", common::rectangle(52.0, 38.0)), 1)],
        insert: InsertSpec::CradlePlatform(CradlePlatformSpec {
            deck_height_mm: 40.0,
            margin_mm: 12.0,
            align: Align::Center,
            placements,
        }),
        outer_box: Some(BoxSpec {
            style: BoxStyle::Mailer,
            material: Material::starting_values(Board::EFlute),
            inside_mm: Some(BoxSize {
                length_mm: 160.0,
                width_mm: 100.0,
                height_mm: 120.0,
            }),
        }),
    }
}

fn with_box((design, style, board): (Design, BoxStyle, Board)) -> Design {
    Design {
        outer_box: Some(BoxSpec {
            style,
            material: Material::starting_values(board),
            inside_mm: None,
        }),
        ..design
    }
}

#[test]
fn default_designs_raise_nothing() {
    let fixture: Design =
        serde_json::from_str(include_str!("fixtures/cradle_mailer.json")).unwrap();
    let mut designs = vec![common::cradle_design(), fixture, common::grid_design()];
    // Every board that holds a 10 mm margin (C-flute needs a 12 mm web), in both boxes.
    for board in [Board::Chipboard, Board::EFlute, Board::BFlute] {
        let unplaced = card((common::card_design().products, Vec::new()));
        for (insert, style) in [
            (common::grid_design(), BoxStyle::Shipping),
            (common::cradle_design(), BoxStyle::Mailer),
            (unplaced, BoxStyle::Mailer),
        ] {
            let mut design = with_box((insert, style, board));
            design.material = Material::starting_values(board);
            designs.push(design);
        }
    }
    for design in &designs {
        let found = warnings(design);
        assert!(found.is_empty(), "{:#?}", listed(&found));
    }
}

#[test]
fn holes_closer_than_a_web_are_flagged() {
    // 20 mm squares: 22 mm holes. Centres 24 mm apart leave 2 mm of board; 10 mm apart
    // the holes overlap.
    let squares = product(("square", common::rectangle(20.0, 20.0)), 2);
    for (spacing, gap) in [(24.0, 2.0), (10.0, 0.0)] {
        let design = card((
            vec![squares.clone()],
            vec![
                placed(("square", 30.0, 30.0)),
                placed(("square", 30.0 + spacing, 30.0)),
            ],
        ));
        let found = warnings(&design);
        let close = only(&found, |rule| matches!(rule, Rule::HolesTooClose { .. }));
        assert_eq!(close.len(), 1, "{:#?}", listed(&found));
        let Rule::HolesTooClose { gap_mm, min_mm } = close[0].rule else {
            unreachable!()
        };
        assert!((gap_mm - gap).abs() < 1e-3, "gap {gap_mm}");
        assert_eq!(min_mm, 3.0);
        let at = &close[0].at;
        assert_eq!(
            (at.part, at.piece, at.placements.as_slice()),
            (Part::Insert, Some(0), &[0, 1][..])
        );
        // The spot covers both holes.
        let area = at.area_mm.unwrap();
        assert!((area.length_mm() - (22.0 + spacing)).abs() < 1e-3);
    }
}

#[test]
fn holes_near_or_past_the_edge_are_flagged() {
    // The sample card pins the gloss so its hole runs off the card's left edge.
    let design = common::card_design();
    let mut design = design;
    design
        .outer_box
        .as_mut()
        .unwrap()
        .inside_mm
        .as_mut()
        .unwrap()
        .height_mm = 100.0;
    let found = warnings(&design);
    assert_eq!(found.len(), 1, "{:#?}", listed(&found));
    assert_eq!(
        found[0].rule,
        Rule::HoleNearEdge {
            gap_mm: 0.0,
            min_mm: 3.0
        }
    );
    assert_eq!(found[0].at.placements, [0]);
    assert!(found[0].at.area_mm.unwrap().min.x < 0.0, "past the edge");

    // C-flute needs 12 mm of board at the edge; a 10 mm margin is short of it.
    let mut c_flute = card((common::card_design().products, Vec::new()));
    c_flute.material = Material::starting_values(Board::CFlute);
    let found = warnings(&c_flute);
    assert_eq!(found.len(), 2, "{:#?}", listed(&found));
    for warning in &found {
        let Rule::HoleNearEdge { gap_mm, min_mm } = warning.rule else {
            panic!("{}", warning.rule)
        };
        assert!(
            (gap_mm - 10.0).abs() < 1e-3 && min_mm == 12.0,
            "{}",
            warning.rule
        );
    }
}

#[test]
fn product_holes_keep_clear_of_creases_but_lock_slots_may_not() {
    // The mailer's base slots sit 1.5 boards from the outer end walls' creases, under
    // the 2-board rule, on purpose: they aren't flagged.
    let design = common::cradle_design();
    let packaging = generate(&design);
    let mailer = &packaging.outer_box.as_ref().unwrap().pieces[0];
    let near = mailer
        .holes
        .iter()
        .flat_map(|hole| {
            Polygon(hole.flatten(CURVE_TOLERANCE_MM))
                .edges()
                .collect::<Vec<_>>()
        })
        .flat_map(|edge| {
            mailer
                .creases
                .iter()
                .map(move |crease| line_of(crease).distance_to_line(edge))
        })
        .fold(f64::INFINITY, f64::min);
    assert!(near < 2.0 * 1.6, "a slot sits {near} mm from a crease");
    let found = checked((&packaging, &design, &CheckOptions::default()));
    assert!(found.is_empty(), "{:#?}", listed(&found));

    // A product dragged next to the deck's long edge: its hole runs into the crease.
    let found = warnings(&cradle(vec![placed(("perfume", 78.5, 20.0))]));
    let crease = only(&found, |rule| matches!(rule, Rule::HoleNearCrease { .. }));
    assert_eq!(crease.len(), 1, "{:#?}", listed(&found));
    assert_eq!(crease[0].at.placements, [0]);
    let Rule::HoleNearCrease { min_mm, .. } = crease[0].rule else {
        unreachable!()
    };
    assert_eq!(min_mm, 3.0);
}

/// A straight crease as a line.
fn line_of(path: &Path) -> Line {
    let to = match path.segments.first() {
        Some(Segment::Line { to }) => *to,
        other => panic!("not a straight crease: {other:?}"),
    };
    Line {
        from: path.start,
        to,
    }
}

#[test]
fn cradle_products_below_the_deck_or_over_a_leg_are_flagged() {
    // Centred on the 157 × 97 mm deck: clear of the legs, and 50 mm tall over a 40 mm deck.
    let centred = cradle(vec![placed(("perfume", 78.5, 48.5))]);
    assert!(
        warnings(&centred).is_empty(),
        "{:#?}",
        listed(&warnings(&centred))
    );

    let mut short = centred.clone();
    short.products[0].height_mm = 35.0;
    let found = warnings(&short);
    assert_eq!(found.len(), 1, "{:#?}", listed(&found));
    assert_eq!(
        found[0].rule,
        Rule::ProductBelowDeck {
            product_id: "perfume".to_owned(),
            height_mm: 35.0,
            deck_height_mm: 40.0
        }
    );
    assert_eq!(found[0].at.placements, [0]);

    // The legs reach 6.9 mm in under each long edge (to the riser's inner face).
    let near_edge = warnings(&cradle(vec![placed(("perfume", 78.5, 20.0))]));
    let over = only(&near_edge, |rule| {
        matches!(rule, Rule::ProductOverLeg { .. })
    });
    assert_eq!(over.len(), 1, "{:#?}", listed(&near_edge));
    assert_eq!(over[0].at.placements, [0]);
    // The spot is on the piece's drawing, which unfolds the back leg above the deck: the
    // product starts 1 mm in from the footprint's edge, which is half a board (0.75 mm)
    // outside the deck panel's crease.
    let packaging = generate(&cradle(vec![placed(("perfume", 78.5, 20.0))]));
    let deck = packaging.insert.pieces[0].panels[0]
        .outline
        .bounds()
        .unwrap();
    let area = over[0].at.area_mm.unwrap();
    assert!(
        (area.min.y - (deck.min.y - 0.75) - 1.0).abs() < 1e-6,
        "{area:?} vs deck {deck:?}"
    );
}

#[test]
fn slots_must_fit_the_board() {
    let slot_rules = |found: &[Warning]| {
        only(found, |rule| {
            matches!(rule, Rule::SlotTooTight { .. } | Rule::SlotTooLoose { .. })
        })
        .iter()
        .map(|w| {
            (
                w.at.part,
                w.at.piece,
                matches!(w.rule, Rule::SlotTooTight { .. }),
            )
        })
        .collect::<Vec<_>>()
    };

    let mut grid = common::grid_design();
    grid.material.slot_tolerance_mm = 0.0;
    assert_eq!(
        slot_rules(&warnings(&grid)),
        [(Part::Insert, Some(0), true), (Part::Insert, Some(1), true)]
    );

    let mut loose = common::cradle_design();
    loose.material.slot_tolerance_mm = 2.0;
    let found = warnings(&loose);
    assert_eq!(slot_rules(&found), [(Part::Insert, Some(0), false)]);
    let loose_rule = only(&found, |rule| matches!(rule, Rule::SlotTooLoose { .. }));
    assert_eq!(
        loose_rule[0].rule,
        Rule::SlotTooLoose {
            slot_mm: 3.5,
            board_mm: 1.5
        }
    );

    let mut tight_mailer = common::cradle_design();
    tight_mailer
        .outer_box
        .as_mut()
        .unwrap()
        .material
        .slot_tolerance_mm = 0.0;
    assert_eq!(
        slot_rules(&warnings(&tight_mailer)),
        [(Part::OuterBox, Some(0), true)]
    );

    // The card has no slots, and the shipping box's flap slots take no board.
    let mut plain = with_box((
        card((common::card_design().products, Vec::new())),
        BoxStyle::Shipping,
        Board::BFlute,
    ));
    plain.material.slot_tolerance_mm = 0.0;
    plain.outer_box.as_mut().unwrap().material.slot_tolerance_mm = 0.0;
    assert!(slot_rules(&warnings(&plain)).is_empty());
}

#[test]
fn unusual_board_thickness_is_flagged() {
    let mut design = common::cradle_design();
    design.material.thickness_mm = 3.0;
    design.outer_box.as_mut().unwrap().material.thickness_mm = 3.0;
    let found = only(&warnings(&design), |rule| {
        matches!(rule, Rule::UnusualThickness { .. })
    });
    let parts: Vec<_> = found.iter().map(|w| (w.at.part, w.at.piece)).collect();
    assert_eq!(parts, [(Part::Insert, None), (Part::OuterBox, None)]);
    let Rule::UnusualThickness {
        board,
        thickness_mm,
        typical,
    } = found[0].rule
    else {
        unreachable!()
    };
    assert_eq!(
        (board, thickness_mm, typical.min, typical.max),
        (Board::Chipboard, 3.0, 1.0, 2.5)
    );
}

#[test]
fn features_under_a_millimetre_are_flagged() {
    // Paperboard slots are 0.5 + 0.3 = 0.8 mm wide: two in the long divider, one in
    // each short divider (one piece, cut twice).
    let mut paper = common::grid_design();
    paper.material = Material::starting_values(Board::Paperboard);
    let found = warnings(&paper);
    assert_eq!(found.len(), 3, "{:#?}", listed(&found));
    for warning in &found {
        let Rule::TooSmall {
            feature,
            size_mm,
            min_mm,
        } = warning.rule
        else {
            panic!("{}", warning.rule)
        };
        assert_eq!((feature, min_mm), (SmallFeature::CutOut, 1.0));
        assert!((0.7..0.85).contains(&size_mm), "{size_mm}");
    }
    let pieces: Vec<_> = found.iter().map(|w| w.at.piece).collect();
    assert_eq!(pieces, [Some(0), Some(0), Some(1)]);

    // Holes too: a paperboard cradle's four lock slots are plain 0.8 mm rectangles.
    let mut paper_cradle = common::cradle_design();
    paper_cradle.material = Material::starting_values(Board::Paperboard);
    paper_cradle.outer_box = None;
    let found = warnings(&paper_cradle);
    assert_eq!(found.len(), 4, "{:#?}", listed(&found));
    assert!(found.iter().all(|w| matches!(
        w.rule,
        Rule::TooSmall {
            feature: SmallFeature::CutOut,
            ..
        }
    )));

    // A product with a narrow V cut into it: the board left in the V's tip, inside the
    // hole, tapers to a point.
    let v = Polygon(
        [
            [0.0, 0.0],
            [40.0, 0.0],
            [40.0, 40.0],
            [22.0, 40.0],
            [20.0, 10.0],
            [18.0, 40.0],
            [0.0, 40.0],
        ]
        .map(Point::from)
        .to_vec(),
    );
    let design = card((vec![product(("v", v), 1)], Vec::new()));
    let found = warnings(&design);
    let spikes = only(&found, |rule| {
        matches!(
            rule,
            Rule::TooSmall {
                feature: SmallFeature::Board,
                ..
            }
        )
    });
    assert_eq!(spikes.len(), 1, "{:#?}", listed(&found));
    // In the V: the product's middle (x = 20 + the 10 mm margin + 1 mm clearance).
    let area = spikes[0].at.area_mm.unwrap();
    assert!((area.center().x - 31.0).abs() < 1.0, "{area:?}");
}

#[test]
fn panels_mostly_holes_are_flagged() {
    // A 100 mm square on a card with 3 mm margins: the hole is 102 mm with round corners.
    let mut design = card((
        vec![product(("tile", common::rectangle(100.0, 100.0)), 1)],
        Vec::new(),
    ));
    if let InsertSpec::CutoutCard(spec) = &mut design.insert {
        spec.margin_mm = 3.0;
    }
    let found = warnings(&design);
    assert_eq!(found.len(), 1, "{:#?}", listed(&found));
    let hole = 100.0 * 100.0 + 4.0 * 100.0 + std::f64::consts::PI;
    let Rule::WeakPanel {
        cut_out_share,
        max_share,
    } = found[0].rule
    else {
        panic!("{}", found[0].rule)
    };
    assert!(
        (cut_out_share - hole / (108.0 * 108.0)).abs() < 1e-3,
        "{cut_out_share}"
    );
    assert_eq!(max_share, 0.6);
}

#[test]
fn pieces_must_fit_the_sheet_either_way_round() {
    let design = common::cradle_design();
    let packaging = generate(&design);
    let size = |piece: &insert_core::Piece| {
        let bounds = Polygon(piece.outline.flatten(CURVE_TOLERANCE_MM))
            .bounds()
            .unwrap();
        (bounds.length_mm(), bounds.width_mm())
    };
    let cradle = size(&packaging.insert.pieces[0]);
    let mailer = size(&packaging.outer_box.as_ref().unwrap().pieces[0]);
    assert!(cradle.1 > cradle.0 && mailer.0 > cradle.1);

    // The cradle fits only turned; the mailer doesn't fit at all.
    let sheet = SheetSize {
        length_mm: cradle.1 + 1.0,
        width_mm: cradle.0 + 1.0,
    };
    let options = CheckOptions { sheet: Some(sheet) };
    let found = checked((&packaging, &design, &options));
    assert_eq!(found.len(), 1, "{:#?}", listed(&found));
    assert_eq!(found[0].at.part, Part::OuterBox);
    assert_eq!(
        found[0].rule,
        Rule::TooBigForSheet {
            length_mm: mailer.0,
            width_mm: mailer.1,
            sheet
        }
    );
    // Without a sheet, nothing to compare.
    assert!(checked((&packaging, &design, &CheckOptions::default())).is_empty());
}

#[test]
fn cut_line_faults_are_flagged() {
    let design = common::cradle_design();
    let clean = generate(&design);
    let kinds = |packaging: &Packaging| -> Vec<Rule> {
        checked((packaging, &design, &CheckOptions::default()))
            .into_iter()
            .map(|w| w.rule)
            .collect()
    };

    let mut open = clean.clone();
    open.insert.pieces[0].outline.closed = false;
    assert_eq!(kinds(&open), [Rule::OpenCut]);

    // A crease drawn twice.
    let mut doubled = clean.clone();
    let crease = doubled.insert.pieces[0].creases[0].clone();
    let length = line_of(&crease).from.distance_to(line_of(&crease).to);
    doubled.insert.pieces[0].creases.push(crease);
    let found = kinds(&doubled);
    assert_eq!(found.len(), 1, "{found:?}");
    let Rule::DuplicateLine { length_mm } = found[0] else {
        panic!("{:?}", found[0])
    };
    assert!((length_mm - length).abs() < 1e-6);

    // A crease drawn straight through the deck's product hole.
    let mut crossed = clean.clone();
    let piece = &mut crossed.insert.pieces[0];
    let hole = Polygon(piece.holes[0].flatten(CURVE_TOLERANCE_MM));
    let deck = piece.panels[0].outline.bounds().unwrap();
    let middle = hole.bounds().unwrap().center().y;
    piece.creases.push(Path {
        start: Point::new(deck.min.x, middle),
        segments: vec![Segment::Line {
            to: Point::new(deck.max.x, middle),
        }],
        closed: false,
    });
    // It also runs through a product hole, which the crease rule flags too.
    let found = kinds(&crossed);
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(found.contains(&Rule::CreaseCrossesCut));
    assert!(found.contains(&Rule::HoleNearCrease {
        gap_mm: 0.0,
        min_mm: 3.0
    }));
}

#[test]
fn warnings_serialize_with_their_kind_and_spot() {
    let mut design = common::card_design();
    design
        .outer_box
        .as_mut()
        .unwrap()
        .inside_mm
        .as_mut()
        .unwrap()
        .height_mm = 100.0;
    let warning = warnings(&design).remove(0);
    let value = serde_json::to_value(&warning).unwrap();
    assert_eq!(value["rule"]["kind"], "hole_near_edge");
    assert_eq!(value["at"]["part"], "insert");
    assert_eq!(value["at"]["placements"], serde_json::json!([0]));
    assert!(value["at"]["area_mm"]["min"].is_array());
    let back: Warning = serde_json::from_value(value).unwrap();
    assert_eq!(back, warning);
    assert_eq!(
        warning.rule.to_string(),
        "a hole reaches the edge; keep holes at least 3.0 mm in"
    );
}
