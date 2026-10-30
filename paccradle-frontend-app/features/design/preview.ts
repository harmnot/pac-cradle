import { NO_SHEET, type InsertCore } from "./insert-core";
import type {
  Design,
  FitColor,
  Mount,
  Packaging,
  Path,
  Piece,
  Placement,
  Point,
  Polygon,
  ProductFit,
  Warning,
} from "./types";

/**
 * What the design pages draw from the engine (M1-E4): one frame of the packaging and its
 * fits, the assembled insert seen from above in its box, and the flat pieces laid side by
 * side. Pure geometry over the engine's JSON, all in mm, x right and y down the page.
 */

/** One generated frame, or the engine's reason it can't be made. */
export type PreviewFrame =
  | {
      readonly kind: "ok";
      readonly packaging: Packaging;
      readonly fits: readonly ProductFit[];
      /** What to fix before cutting (CLAUDE.md §6.5); the editor lists them (M1-E5). */
      readonly warnings: readonly Warning[];
    }
  | { readonly kind: "error"; readonly message: string };

/**
 * Generates the design, checks every product copy's fit and runs the manufacturability
 * checks. For discrete choices: a dragged slider should skip the checks until it settles.
 */
export function runPreview({ core, design }: { core: InsertCore; design: Design }): PreviewFrame {
  const packaging = core.generate(design);
  if (packaging.kind === "error") {
    return packaging;
  }
  const fits = core.fits(design);
  if (fits.kind === "error") {
    return fits;
  }
  const warnings = core.warnings({ design, options: NO_SHEET });
  return {
    kind: "ok",
    packaging: packaging.value,
    fits: fits.value,
    warnings: warnings.kind === "ok" ? warnings.value : [],
  };
}

// ---------------------------------------------------------------- shapes

/** An axis-aligned rectangle. */
export interface Rect {
  readonly x: number;
  readonly y: number;
  readonly width: number;
  readonly height: number;
}

/** The box round a set of points (zero size when there are none). */
export function boundsOf(points: readonly Point[]): Rect {
  if (points.length === 0) {
    return { x: 0, y: 0, width: 0, height: 0 };
  }
  const xs = points.map((point) => point[0]);
  const ys = points.map((point) => point[1]);
  const [minX, minY] = [Math.min(...xs), Math.min(...ys)];
  return { x: minX, y: minY, width: Math.max(...xs) - minX, height: Math.max(...ys) - minY };
}

/** Grows a rectangle by `by` on every side. */
export function expand({ rect, by }: { rect: Rect; by: number }): Rect {
  return { x: rect.x - by, y: rect.y - by, width: rect.width + 2 * by, height: rect.height + 2 * by };
}

/** The points a path passes through (the preview's paths are straight lines only). */
export function pathPoints(path: Path): readonly Point[] {
  return [path.start, ...path.segments.map((segment) => (segment.kind === "line" ? segment.to : segment.center))];
}

/** `point` turned by `deg` about the origin, clockwise on screen when positive. */
function turn({ point, deg }: { point: Point; deg: number }): Point {
  const rad = (deg * Math.PI) / 180;
  const [cos, sin] = [Math.cos(rad), Math.sin(rad)];
  return [point[0] * cos - point[1] * sin, point[0] * sin + point[1] * cos];
}

/**
 * A product's outline where a placement puts it: the centre of its bounding box (taken
 * before turning) at `center_mm`, turned by `rotation_deg` (as `ProductHole::placed_outline`).
 */
export function placedOutline({ contour, placement }: { contour: Polygon; placement: Placement }): Polygon {
  const box = boundsOf(contour);
  const pivot: Point = [box.x + box.width / 2, box.y + box.height / 2];
  return contour.map((point) => {
    const turned = turn({ point: [point[0] - pivot[0], point[1] - pivot[1]], deg: placement.rotation_deg });
    return [turned[0] + placement.center_mm[0], turned[1] + placement.center_mm[1]] as const;
  });
}

// ---------------------------------------------------------------- the view from above

/** Board seen from above: a flat panel with its holes, or an upright strip's top edge. */
export type BoardShape =
  | { readonly kind: "panel"; readonly outline: Polygon; readonly holes: readonly Polygon[] }
  | { readonly kind: "strip"; readonly rect: Rect };

/** A product copy seen from above, coloured by its fit. */
export interface PlacedProduct {
  readonly outline: Polygon;
  readonly color: FitColor;
}

/** The assembled insert in its box, from above, in the insert's frame (top-left at 0, 0). */
export interface PlanView {
  readonly insert: Rect;
  readonly board: readonly BoardShape[];
  readonly products: readonly PlacedProduct[];
  /** The box's inside and its walls' outer edge (the board's thickness out), when there's a box. */
  readonly box: { readonly inside: Rect; readonly walls: Rect } | null;
  /** Everything drawn, for the drawing's frame. */
  readonly extent: Rect;
}

/** The x range a piece spans in its own drawing (an upright strip's length). */
function pieceSpan(piece: Piece): { readonly min: number; readonly max: number } {
  const points = piece.panels.flatMap((panel) => panel.outline);
  const box = boundsOf(points.length > 0 ? points : pathPoints(piece.outline));
  return { min: box.x, max: box.x + box.width };
}

/** One mount's board as seen from above. Flat pieces show their root panel (a cradle's deck). */
function boardShape({ mount, piece, thickness }: { mount: Mount; piece: Piece; thickness: number }): BoardShape {
  const [x, y] = mount.at_mm;
  const shift = (point: Point): Point => [point[0] + x, point[1] + y];
  switch (mount.stance) {
    case "flat": {
      const root = piece.panels[0];
      return root === undefined
        ? { kind: "panel", outline: pathPoints(piece.outline).map(shift), holes: [] }
        : { kind: "panel", outline: root.outline.map(shift), holes: root.holes.map((hole) => hole.map(shift)) };
    }
    case "upright_along_x": {
      const span = pieceSpan(piece);
      return { kind: "strip", rect: { x: x + span.min, y: y - thickness / 2, width: span.max - span.min, height: thickness } };
    }
    case "upright_along_y": {
      const span = pieceSpan(piece);
      return { kind: "strip", rect: { x: x - thickness / 2, y: y + span.min, width: thickness, height: span.max - span.min } };
    }
  }
}

/** The insert, its products and its box, as seen from above. */
export function planView({ design, packaging, fits }: { design: Design; packaging: Packaging; fits: readonly ProductFit[] }): PlanView {
  const { insert } = packaging;
  const thickness = design.material.thickness_mm;
  const board = insert.mounts.flatMap((mount) => {
    const piece = insert.pieces[mount.piece];
    return piece === undefined ? [] : [boardShape({ mount, piece, thickness })];
  });
  const colors = new Map(fits.map((fit) => [fit.placement, fit.color]));
  const products = insert.placements.flatMap((placement, index) => {
    const product = design.products.find((candidate) => candidate.id === placement.product_id);
    return product === undefined
      ? []
      : [{ outline: placedOutline({ contour: product.contour_mm, placement }), color: colors.get(index) ?? "green" }];
  });
  const insertRect: Rect = { x: 0, y: 0, width: insert.length_mm, height: insert.width_mm };
  const outer = packaging.outer_box;
  const box =
    outer === null || design.outer_box === null
      ? null
      : (() => {
          const inside: Rect = {
            x: (insert.length_mm - outer.inside_mm.length_mm) / 2,
            y: (insert.width_mm - outer.inside_mm.width_mm) / 2,
            width: outer.inside_mm.length_mm,
            height: outer.inside_mm.width_mm,
          };
          return { inside, walls: expand({ rect: inside, by: design.outer_box.material.thickness_mm }) };
        })();
  const drawn = [
    ...rectCorners(insertRect),
    ...products.flatMap((product) => product.outline),
    ...(box === null ? [] : rectCorners(box.walls)),
  ];
  return { insert: insertRect, board, products, box, extent: boundsOf(drawn) };
}

function rectCorners(rect: Rect): readonly Point[] {
  return [
    [rect.x, rect.y],
    [rect.x + rect.width, rect.y + rect.height],
  ];
}

// ---------------------------------------------------------------- the flat pieces

/** How much wider than a square the rows of flat pieces run (the preview panel is landscape). */
const ROW_STRETCH = 1.5;

/** A piece placed in the row of flat pieces. */
export interface LaidPiece {
  readonly piece: Piece;
  /** Whether it belongs to the outer box (drawn on kraft) or the insert. */
  readonly part: "insert" | "outer_box";
  /** Moves the piece's own drawing into the row. */
  readonly shift: Point;
  readonly frame: Rect;
}

/** The pieces in rows no wider than about the widest piece's share, and the space they take. */
export function layOutPieces({ packaging, gapMm }: { packaging: Packaging; gapMm: number }): {
  readonly pieces: readonly LaidPiece[];
  readonly extent: Rect;
} {
  const all = [
    ...packaging.insert.pieces.map((piece) => ({ piece, part: "insert" as const })),
    ...(packaging.outer_box?.pieces ?? []).map((piece) => ({ piece, part: "outer_box" as const })),
  ];
  const sized = all.map((item) => ({ ...item, box: boundsOf(pathPoints(item.piece.outline)) }));
  const widest = Math.max(0, ...sized.map((item) => item.box.width));
  const area = sized.reduce((sum, item) => sum + (item.box.width + gapMm) * (item.box.height + gapMm), 0);
  // Rows a little wider than the square the pieces would fill, never narrower than the widest.
  const rowWidth = Math.max(widest, Math.sqrt(area) * ROW_STRETCH);
  const laid: LaidPiece[] = [];
  let [x, y, rowHeight] = [0, 0, 0];
  for (const item of sized) {
    if (x > 0 && x + item.box.width > rowWidth) {
      [x, y, rowHeight] = [0, y + rowHeight + gapMm, 0];
    }
    laid.push({
      piece: item.piece,
      part: item.part,
      shift: [x - item.box.x, y - item.box.y],
      frame: { x, y, width: item.box.width, height: item.box.height },
    });
    x += item.box.width + gapMm;
    rowHeight = Math.max(rowHeight, item.box.height);
  }
  return { pieces: laid, extent: boundsOf(laid.flatMap((item) => rectCorners(item.frame))) };
}

// ---------------------------------------------------------------- fit

/** How many copies fit well, tightly, or not at all, and the smallest clearance. */
export interface FitSummary {
  readonly green: number;
  readonly yellow: number;
  readonly red: number;
  readonly tightestMm: number;
}

/** Counts the fits by colour. */
export function fitSummary(fits: readonly ProductFit[]): FitSummary {
  const count = (color: FitColor) => fits.filter((fit) => fit.color === color).length;
  return {
    green: count("green"),
    yellow: count("yellow"),
    red: count("red"),
    tightestMm: fits.reduce((least, fit) => Math.min(least, fit.clearance_mm), Number.POSITIVE_INFINITY),
  };
}

/** SVG points for a polygon. */
export function polygonPoints(polygon: Polygon): string {
  return polygon.map((point) => `${point[0]},${point[1]}`).join(" ");
}

/** SVG path data for a polygon with holes (drawn with `fill-rule: evenodd`). */
export function ringsData({ outline, holes }: { outline: Polygon; holes: readonly Polygon[] }): string {
  return [outline, ...holes]
    .filter((ring) => ring.length > 0)
    .map((ring) => `M${ring.map((point) => `${point[0]} ${point[1]}`).join("L")}Z`)
    .join("");
}
