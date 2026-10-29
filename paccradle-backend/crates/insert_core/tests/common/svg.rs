//! A plain SVG drawing of everything a design generates, for the golden tests (M1-A9).
//!
//! One drawing per design: every piece flat, a top view of the insert in its box with
//! the products coloured by fit, and the sizes, fits and warnings as text. It's for
//! people to look at and for the tests to compare, not the production export (M1-B1):
//! no cutter layers, and box pieces aren't mirrored.
//!
//! Drawn in mm, x right and y down like the engine. Creases are grey dashes; each hinge
//! is drawn on top in blue (valley) or red (mountain) with its fold stage, so a hinge
//! that turns past its crease shows where. Numbers are written to 0.001 mm.

use std::fmt::Write;

use insert_core::{
    Bounds, CheckOptions, Design, FitColor, FoldDirection, Hinge, Packaging, Path, Piece, Point,
    Polygon, ProductFit, Segment, Stance, Transform, Warning,
};

/// Space between blocks and round the drawing.
const GAP_MM: f64 = 12.0;
/// Room above each block for its label.
const LABEL_MM: f64 = 7.0;
/// Text size and line spacing.
const FONT_MM: f64 = 4.0;
const LINE_MM: f64 = 6.0;
/// Rough width of one character of text and of a bold label, to size the blocks.
const CHAR_MM: f64 = 2.1;
const LABEL_CHAR_MM: f64 = 2.6;
/// Rows of blocks wrap past this width.
const ROW_MM: f64 = 700.0;

const STYLE: &str = "\
text { font-family: Helvetica, Arial, sans-serif; fill: #222; }
.label { font-size: 4.5px; font-weight: bold; }
.note { font-size: 4px; }
.stage { font-size: 2.6px; }
.cut { fill: none; stroke: #111; stroke-width: 0.35; }
.crease { fill: none; stroke: #999; stroke-width: 0.3; stroke-dasharray: 3 1.5; }
.perf { fill: none; stroke: #111; stroke-width: 0.3; stroke-dasharray: 0.8 1.2; }
.valley { stroke: #1f6fd1; stroke-width: 0.5; }
.mountain { stroke: #d1421f; stroke-width: 0.5; }
.valley-text { fill: #1f6fd1; }
.mountain-text { fill: #d1421f; }
.board { fill: #e8d2ac; stroke: #8a6a3a; stroke-width: 0.3; fill-rule: evenodd; }
.box { fill: none; stroke: #8a6a3a; stroke-width: 0.4; stroke-dasharray: 4 2; }
.green { fill: #3a9d4a; fill-opacity: 0.35; stroke: #2a7a37; stroke-width: 0.3; }
.yellow { fill: #e0b020; fill-opacity: 0.45; stroke: #a07a00; stroke-width: 0.3; }
.red { fill: #d33; fill-opacity: 0.45; stroke: #a11; stroke-width: 0.3; }
";

/// Draws `design`, generated, under the heading `title`. Panics when the design doesn't
/// generate: every golden sample must.
pub fn render((design, title): (&Design, &str)) -> String {
    let packaging = design
        .generate()
        .unwrap_or_else(|error| panic!("{title}: generate: {error}"));
    let fits = packaging
        .fits(design)
        .unwrap_or_else(|error| panic!("{title}: fits: {error}"));
    let warnings = packaging
        .warnings((design, &CheckOptions::default()))
        .unwrap_or_else(|error| panic!("{title}: warnings: {error}"));

    let mut blocks: Vec<Block> = packaging
        .insert
        .pieces
        .iter()
        .enumerate()
        .map(|(index, piece)| piece_block((piece, &format!("Insert piece {index}"))))
        .collect();
    blocks.push(top_view((design, &packaging, &fits)));
    if let Some(outer_box) = &packaging.outer_box {
        blocks.extend(
            outer_box
                .pieces
                .iter()
                .enumerate()
                .map(|(index, piece)| piece_block((piece, &format!("Box piece {index}")))),
        );
    }
    blocks.push(text_block(summary((
        design,
        &packaging,
        (fits.as_slice(), warnings.as_slice()),
    ))));
    document((title, blocks))
}

/// A drawn part of the page, with its top-left corner at (0, 0).
struct Block {
    size: (f64, f64),
    svg: String,
}

/// Lays the blocks out in rows under the title.
fn document((title, blocks): (&str, Vec<Block>)) -> String {
    let mut body = String::new();
    let (mut x, mut y, mut row_height) = (GAP_MM, GAP_MM + LABEL_MM + LINE_MM, 0.0_f64);
    let mut width = 0.0_f64;
    for block in blocks {
        let (block_width, block_height) = block.size;
        if x > GAP_MM && x + block_width > ROW_MM {
            x = GAP_MM;
            y += row_height + GAP_MM;
            row_height = 0.0;
        }
        let _ = writeln!(
            body,
            "<g transform=\"translate({} {})\">\n{}</g>",
            num(x),
            num(y),
            block.svg
        );
        x += block_width + GAP_MM;
        width = width.max(x);
        row_height = row_height.max(block_height);
    }
    let height = y + row_height + GAP_MM;
    let (width, height) = (num(width), num(height));
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}mm\" height=\"{height}mm\" \
         viewBox=\"0 0 {width} {height}\">\n<style>\n{STYLE}</style>\n\
         <rect width=\"{width}\" height=\"{height}\" fill=\"#fff\"/>\n\
         <text class=\"label\" x=\"{gap}\" y=\"{title_y}\">{title}</text>\n{body}</svg>\n",
        gap = num(GAP_MM),
        title_y = num(GAP_MM + FONT_MM),
        title = escape(title),
    )
}

/// One piece lying flat: cut lines, creases, perforations and hinges, with its name,
/// how many to cut and its size.
fn piece_block((piece, heading): (&Piece, &str)) -> Block {
    let outline = piece.outline.flatten(0.01);
    let bounds = Bounds::of(outline.iter().copied())
        .unwrap_or_else(|| panic!("{heading}: piece `{}` has no outline", piece.name));
    let label = format!(
        "{heading}: {} × {} · {} × {} mm",
        piece.name,
        piece.quantity,
        num(bounds.length_mm()),
        num(bounds.width_mm()),
    );
    let mut svg = label_text(&label);
    let _ = writeln!(
        svg,
        "<g transform=\"translate({} {})\">",
        num(-bounds.min.x),
        num(LABEL_MM - bounds.min.y),
    );
    for crease in &piece.creases {
        let _ = writeln!(svg, "<path class=\"crease\" d=\"{}\"/>", path_data(crease));
    }
    for (index, hinge) in piece.hinges.iter().enumerate() {
        svg.push_str(&hinge_mark((index, hinge)));
    }
    for perforation in &piece.perforations {
        let _ = writeln!(
            svg,
            "<path class=\"perf\" d=\"{}\"/>",
            path_data(perforation)
        );
    }
    for cut in std::iter::once(&piece.outline).chain(&piece.holes) {
        let _ = writeln!(svg, "<path class=\"cut\" d=\"{}\"/>", path_data(cut));
    }
    svg.push_str("</g>\n");
    Block {
        size: (
            bounds.length_mm().max(label_width(&label)),
            LABEL_MM + bounds.width_mm(),
        ),
        svg,
    }
}

/// A block's bold label, above its drawing.
fn label_text(label: &str) -> String {
    format!(
        "<text class=\"label\" y=\"{}\">{}</text>\n",
        num(FONT_MM),
        escape(label)
    )
}

fn label_width(label: &str) -> f64 {
    LABEL_CHAR_MM * label.chars().count() as f64
}

/// A hinge's axis in its fold colour, labelled with its fold stage, then its angle when
/// it isn't a quarter turn and how far past the crease it turns. Labels sit a third of
/// the way along, or two thirds for odd `index`, so creases side by side stay readable.
fn hinge_mark((index, hinge): (usize, &Hinge)) -> String {
    let class = match hinge.direction {
        FoldDirection::Valley => "valley",
        FoldDirection::Mountain => "mountain",
    };
    let (from, to) = (hinge.axis.from, hinge.axis.to);
    let along = if index % 2 == 0 { 1.0 / 3.0 } else { 2.0 / 3.0 };
    let label_at = Point::new(
        from.x + (to.x - from.x) * along,
        from.y + (to.y - from.y) * along,
    );
    let mut label = format!("s{}", hinge.stage);
    if hinge.fold_deg != 90.0 {
        let _ = write!(label, " {}°", num(hinge.fold_deg));
    }
    if hinge.offset_mm != 0.0 {
        let _ = write!(label, " +{}", num(hinge.offset_mm));
    }
    format!(
        "<line class=\"{class}\" x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\"/>\n\
         <text class=\"stage {class}-text\" x=\"{}\" y=\"{}\">{label}</text>\n",
        num(from.x),
        num(from.y),
        num(to.x),
        num(to.y),
        num(label_at.x + 0.8),
        num(label_at.y - 0.8),
    )
}

/// The insert from above, assembled in the box's inside, with every product copy
/// coloured by its fit and numbered by its placement.
fn top_view((design, packaging, fits): (&Design, &Packaging, &[ProductFit])) -> Block {
    let insert = &packaging.insert;
    // The box's inside sets the view; without a box, the insert does.
    let inside = packaging.outer_box.as_ref().map(|outer_box| {
        let size = outer_box.inside_mm;
        let corner = Point::new(
            (insert.length_mm - size.length_mm) / 2.0,
            (insert.width_mm - size.width_mm) / 2.0,
        );
        Bounds {
            min: corner,
            max: Point::new(corner.x + size.length_mm, corner.y + size.width_mm),
        }
    });
    let view = inside.unwrap_or(Bounds::from_size((insert.length_mm, insert.width_mm)));
    let label = match inside {
        Some(_) => "Top view in the box",
        None => "Top view",
    };
    let mut svg = label_text(label);
    let _ = writeln!(
        svg,
        "<g transform=\"translate({} {})\">",
        num(-view.min.x),
        num(LABEL_MM - view.min.y),
    );
    if let Some(inside) = inside {
        let _ = writeln!(
            svg,
            "<rect class=\"box\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"/>",
            num(inside.min.x),
            num(inside.min.y),
            num(inside.length_mm()),
            num(inside.width_mm()),
        );
    }
    let thickness = design.material.thickness_mm;
    for mount in &insert.mounts {
        let piece = &insert.pieces[mount.piece];
        let [x, y, _] = mount.at_mm;
        let at = Point::new(x, y);
        svg.push_str(&match mount.stance {
            Stance::Flat => flat_from_above((piece, at)),
            Stance::UprightAlongX | Stance::UprightAlongY => {
                upright_from_above((piece, mount.stance, (at, thickness)))
            }
        });
    }
    for (index, placement) in insert.placements.iter().enumerate() {
        let product = design
            .products
            .iter()
            .find(|product| product.id == placement.product_id)
            .unwrap_or_else(|| panic!("no product `{}`", placement.product_id));
        let pivot = product
            .contour_mm
            .bounds()
            .unwrap_or_else(|| panic!("product `{}` has no outline", product.id))
            .center();
        let place = Transform::shift(Point::ORIGIN - pivot).then(Transform {
            turn_deg: placement.rotation_deg,
            shift: placement.center_mm,
        });
        let class = fits
            .iter()
            .find(|fit| fit.placement == index)
            .map_or("red", |fit| color_class(fit.color));
        let _ = writeln!(
            svg,
            "<path class=\"{class}\" d=\"{}\"/>\n<text class=\"note\" x=\"{}\" y=\"{}\">{index}</text>",
            polygon_data(&product.contour_mm.transformed(place)),
            num(placement.center_mm.x - 1.0),
            num(placement.center_mm.y + 1.4),
        );
    }
    svg.push_str("</g>\n");
    Block {
        size: (
            view.length_mm().max(label_width(label)),
            LABEL_MM + view.width_mm(),
        ),
        svg,
    }
}

/// A piece lying flat, seen from above: its root panel (the card, or a cradle's deck).
fn flat_from_above((piece, at): (&Piece, Point)) -> String {
    let Some(root) = piece.panels.first() else {
        return String::new();
    };
    let data: Vec<String> = std::iter::once(&root.outline)
        .chain(&root.holes)
        .map(|ring| polygon_data(&ring.transformed(Transform::shift(at))))
        .collect();
    format!("<path class=\"board\" d=\"{}\"/>\n", data.join(" "))
}

/// A piece standing on edge, seen from above: a strip one board thick along its length.
fn upright_from_above((piece, stance, (at, thickness)): (&Piece, Stance, (Point, f64))) -> String {
    let outline = piece.outline.flatten(0.01);
    let Some(bounds) = Bounds::of(outline) else {
        return String::new();
    };
    let half = thickness / 2.0;
    let (min, max) = match stance {
        Stance::UprightAlongY => (
            Point::new(at.x - half, at.y + bounds.min.x),
            Point::new(at.x + half, at.y + bounds.max.x),
        ),
        _ => (
            Point::new(at.x + bounds.min.x, at.y - half),
            Point::new(at.x + bounds.max.x, at.y + half),
        ),
    };
    format!(
        "<rect class=\"board\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"/>\n",
        num(min.x),
        num(min.y),
        num(max.x - min.x),
        num(max.y - min.y),
    )
}

/// The sizes, fits and warnings, one line each.
fn summary(
    (design, packaging, (fits, warnings)): (&Design, &Packaging, (&[ProductFit], &[Warning])),
) -> Vec<String> {
    let insert = &packaging.insert;
    let material = &design.material;
    let mut lines = vec![format!(
        "Insert: {:?} {} mm · {} × {} × {} mm",
        material.board,
        num(material.thickness_mm),
        num(insert.length_mm),
        num(insert.width_mm),
        num(insert.height_mm),
    )];
    if let (Some(outer_box), Some(spec)) = (&packaging.outer_box, &design.outer_box) {
        let (inside, outside) = (outer_box.inside_mm, outer_box.outside_mm);
        lines.push(format!(
            "Box: {:?} in {:?} {} mm · inside {} × {} × {} · outside {} × {} × {} mm",
            outer_box.style,
            spec.material.board,
            num(spec.material.thickness_mm),
            num(inside.length_mm),
            num(inside.width_mm),
            num(inside.height_mm),
            num(outside.length_mm),
            num(outside.width_mm),
            num(outside.height_mm),
        ));
    }
    lines.push("Fit:".to_owned());
    for fit in fits {
        let product_id = &insert.placements[fit.placement].product_id;
        let clashes = if fit.clashes.is_empty() {
            String::new()
        } else {
            format!(", runs into {:?}", fit.clashes)
        };
        lines.push(format!(
            "  #{} {product_id}: {:?}, {} mm clear (target {} mm){clashes}",
            fit.placement,
            fit.color,
            num(fit.clearance_mm),
            num(fit.target_mm),
        ));
    }
    lines.push(format!("Warnings: {}", warnings.len()));
    for warning in warnings {
        let piece = warning
            .at
            .piece
            .map_or(String::new(), |piece| format!(" piece {piece}"));
        lines.push(format!("  {:?}{piece}: {}", warning.at.part, warning.rule));
    }
    lines
}

/// Lines of text, top to bottom.
fn text_block(lines: Vec<String>) -> Block {
    let mut svg = String::new();
    let mut longest = 0;
    for (index, line) in lines.iter().enumerate() {
        longest = longest.max(line.chars().count());
        let _ = writeln!(
            svg,
            "<text class=\"note\" y=\"{}\" xml:space=\"preserve\">{}</text>",
            num(FONT_MM + LINE_MM * index as f64),
            escape(line),
        );
    }
    Block {
        size: (CHAR_MM * longest as f64, LINE_MM * lines.len() as f64),
        svg,
    }
}

fn color_class(color: FitColor) -> &'static str {
    match color {
        FitColor::Green => "green",
        FitColor::Yellow => "yellow",
        FitColor::Red => "red",
    }
}

/// SVG path data for a path, keeping arcs as arcs.
fn path_data(path: &Path) -> String {
    let mut data = format!("M{} {}", num(path.start.x), num(path.start.y));
    let mut current = path.start;
    for segment in &path.segments {
        match *segment {
            Segment::Line { to } => {
                let _ = write!(data, " L{} {}", num(to.x), num(to.y));
                current = to;
            }
            Segment::Arc { center, sweep_deg } => {
                // SVG can't draw a full circle in one arc, so long arcs go in two halves.
                let parts = if sweep_deg.abs() > 180.0 { 2 } else { 1 };
                let part_deg = sweep_deg / f64::from(parts);
                for _ in 0..parts {
                    current = arc_end((current, center, part_deg));
                    let radius = num(current.distance_to(center));
                    let sweep_flag = u8::from(part_deg > 0.0);
                    let _ = write!(
                        data,
                        " A{radius} {radius} 0 0 {sweep_flag} {} {}",
                        num(current.x),
                        num(current.y),
                    );
                }
            }
        }
    }
    if path.closed {
        data.push_str(" Z");
    }
    data
}

/// Where an arc from `start` round `center` ends after turning `sweep_deg`.
fn arc_end((start, center, sweep_deg): (Point, Point, f64)) -> Point {
    let turn = Transform::shift(Point::ORIGIN - center).then(Transform {
        turn_deg: sweep_deg,
        shift: center,
    });
    turn.apply(start)
}

fn polygon_data(polygon: &Polygon) -> String {
    let points: Vec<String> = polygon
        .points()
        .iter()
        .map(|point| format!("{} {}", num(point.x), num(point.y)))
        .collect();
    format!("M{}Z", points.join(" L"))
}

/// A number to 0.001 mm, without trailing zeros, and never "-0".
pub fn num(value: f64) -> String {
    let text = format!("{value:.3}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    match text {
        "-0" | "" => "0".to_owned(),
        _ => text.to_owned(),
    }
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
