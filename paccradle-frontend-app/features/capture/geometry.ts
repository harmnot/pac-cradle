import { none, some, type Maybe } from "@/lib/maybe";
import type { Point, WireVisionResult } from "./types";

/**
 * Plane geometry for checking an outline over the photo (MILESTONES M1-E3): the photo ↔
 * paper mapping through the sheet's 4 corners, and the checks vision makes on an edited
 * outline (on the paper, not crossing itself, at least 25 mm²), so the editor can show them
 * while the points move. Vision checks again and measures the outline that counts.
 */

/** A 3 × 3 projective transform, by rows. */
export type Matrix3 = readonly [number, number, number, number, number, number, number, number, number];

/** A size in millimetres. */
export interface SizeMm {
  readonly width: number;
  readonly height: number;
}

/** Sheet sizes, portrait (vision's `PaperSize::size_mm`). */
const PAPER_MM: Readonly<Record<string, SizeMm>> = {
  a4: { width: 210, height: 297 },
  letter: { width: 215.9, height: 279.4 },
};

/** Smallest outline vision measures (`segment::MIN_OBJECT_AREA_MM2`). */
export const MIN_OUTLINE_AREA_MM2 = 25;
/** How far inside the sheet's edge a dragged point is kept, so rounding never puts it off. */
const EDGE_INSET_MM = 0.05;
/** Below this, a matrix counts as singular. */
const EPSILON = 1e-12;
/** Shape tests vision uses for "round" (`measure::ROUND_MIN_CIRCULARITY`, `ROUND_MAX_ASPECT`). */
const ROUND_MIN_CIRCULARITY = 0.85;
const ROUND_MAX_ASPECT = 1.1;

/** `[0, 1, …, count − 1]`. */
function range(count: number): readonly number[] {
  return Array.from({ length: count }, (_, index) => index);
}

/** The sheet's size for a result's `paper.size` (`A4`, `Letter`, any case). */
export function paperSizeMm(size: string): Maybe<SizeMm> {
  const found = PAPER_MM[size.toLowerCase()];
  return found === undefined ? none : some(found);
}

/**
 * Solves `matrix · x = values` by Gaussian elimination with partial pivoting; `none` when the
 * system has no single answer.
 */
function solve({ matrix, values }: { matrix: readonly (readonly number[])[]; values: readonly number[] }): Maybe<number[]> {
  const rows = matrix.map((row, index) => [...row, values[index]]);
  const size = rows.length;
  for (const column of range(size)) {
    const pivot = range(size)
      .slice(column)
      .reduce((best, row) => (Math.abs(rows[row][column]) > Math.abs(rows[best][column]) ? row : best), column);
    if (Math.abs(rows[pivot][column]) < EPSILON) {
      return none;
    }
    [rows[column], rows[pivot]] = [rows[pivot], rows[column]];
    for (const row of range(size)) {
      if (row === column) {
        continue;
      }
      const factor = rows[row][column] / rows[column][column];
      rows[row] = rows[row].map((value, index) => value - factor * rows[column][index]);
    }
  }
  return some(rows.map((row, index) => row[size] / row[index]));
}

/** The projective transform taking each `from` corner to the same `to` corner. */
export function homography({ from, to }: { from: readonly Point[]; to: readonly Point[] }): Maybe<Matrix3> {
  if (from.length !== 4 || to.length !== 4) {
    return none;
  }
  const equations = from.flatMap(([x, y], index) => {
    const [u, v] = to[index];
    return [
      { row: [x, y, 1, 0, 0, 0, -u * x, -u * y], value: u },
      { row: [0, 0, 0, x, y, 1, -v * x, -v * y], value: v },
    ];
  });
  const solved = solve({ matrix: equations.map((equation) => equation.row), values: equations.map((equation) => equation.value) });
  if (solved.kind === "none") {
    return none;
  }
  const [a, b, c, d, e, f, g, h] = solved.value;
  return some([a, b, c, d, e, f, g, h, 1]);
}

/** Where `transform` takes `point`; `none` on the transform's horizon. */
export function project({ transform, point }: { transform: Matrix3; point: Point }): Maybe<Point> {
  const [a, b, c, d, e, f, g, h, i] = transform;
  const [x, y] = point;
  const w = g * x + h * y + i;
  if (Math.abs(w) < EPSILON) {
    return none;
  }
  return some([(a * x + b * y + c) / w, (d * x + e * y + f) / w]);
}

/** The inverse transform, by the adjugate; `none` when there is none. */
export function invert(transform: Matrix3): Maybe<Matrix3> {
  const [a, b, c, d, e, f, g, h, i] = transform;
  const determinant = a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g);
  if (Math.abs(determinant) < EPSILON) {
    return none;
  }
  const over = (value: number) => value / determinant;
  return some([
    over(e * i - f * h),
    over(c * h - b * i),
    over(b * f - c * e),
    over(f * g - d * i),
    over(a * i - c * g),
    over(c * d - a * f),
    over(d * h - e * g),
    over(b * g - a * h),
    over(a * e - b * d),
  ]);
}

/** The photo ↔ paper mapping of one result. */
export interface PaperMap {
  /** The sheet, portrait, in mm. */
  readonly size: SizeMm;
  readonly toPaper: Matrix3;
  readonly toPhoto: Matrix3;
}

/** The mapping through the sheet's corners (top-left, top-right, bottom-right, bottom-left). */
export function paperMapOf(paper: WireVisionResult["paper"]): Maybe<PaperMap> {
  const size = paperSizeMm(paper.size);
  if (size.kind === "none") {
    return none;
  }
  const { width, height } = size.value;
  const sheet: readonly Point[] = [
    [0, 0],
    [width, 0],
    [width, height],
    [0, height],
  ];
  const toPaper = homography({ from: paper.corners_px, to: sheet });
  if (toPaper.kind === "none") {
    return none;
  }
  const toPhoto = invert(toPaper.value);
  return toPhoto.kind === "none" ? none : some({ size: size.value, toPaper: toPaper.value, toPhoto: toPhoto.value });
}

/** A photo point on the sheet, in mm from its top-left corner. */
export function toPaperMm({ map, point }: { map: PaperMap; point: Point }): Maybe<Point> {
  return project({ transform: map.toPaper, point });
}

/** Whether a photo point lies on the sheet, edges included (vision's `on_paper`). */
export function onPaper({ map, point }: { map: PaperMap; point: Point }): boolean {
  const mm = toPaperMm({ map, point });
  if (mm.kind === "none") {
    return false;
  }
  const [x, y] = mm.value;
  return x >= 0 && x <= map.size.width && y >= 0 && y <= map.size.height;
}

/** The photo point, moved onto the sheet when it lies off it. */
export function clampToPaper({ map, point }: { map: PaperMap; point: Point }): Point {
  const mm = toPaperMm({ map, point });
  if (mm.kind === "none") {
    return point;
  }
  const [x, y] = mm.value;
  const clamp = (value: number, max: number) => Math.min(Math.max(value, EDGE_INSET_MM), max - EDGE_INSET_MM);
  const inside: Point = [clamp(x, map.size.width), clamp(y, map.size.height)];
  if (inside[0] === x && inside[1] === y) {
    return point;
  }
  const back = project({ transform: map.toPhoto, point: inside });
  return back.kind === "some" ? back.value : point;
}

/** Each edge of a closed outline, from point `i` to the next. */
function edgesOf(points: readonly Point[]): readonly (readonly [Point, Point])[] {
  return points.map((point, index) => [point, points[(index + 1) % points.length]] as const);
}

/** The polygon's area (shoelace), whichever way round it runs. */
export function polygonArea(points: readonly Point[]): number {
  const twice = edgesOf(points).reduce((sum, [[ax, ay], [bx, by]]) => sum + ax * by - bx * ay, 0);
  return Math.abs(twice) / 2;
}

/** The outline's length. */
function perimeter(points: readonly Point[]): number {
  return edgesOf(points).reduce((sum, [[ax, ay], [bx, by]]) => sum + Math.hypot(bx - ax, by - ay), 0);
}

/** Which side of the line through an edge a point lies on: positive one way, negative the other. */
function turn({ edge: [start, end], point }: { edge: readonly [Point, Point]; point: Point }): number {
  return (end[0] - start[0]) * (point[1] - start[1]) - (end[1] - start[1]) * (point[0] - start[0]);
}

/** Whether a point on an edge's line lies between its ends. */
function within({ point, edge: [start, end] }: { point: Point; edge: readonly [Point, Point] }): boolean {
  return (
    point[0] >= Math.min(start[0], end[0]) &&
    point[0] <= Math.max(start[0], end[0]) &&
    point[1] >= Math.min(start[1], end[1]) &&
    point[1] <= Math.max(start[1], end[1])
  );
}

/** Whether two edges cross, or one ends on the other (vision's `segments_meet`). */
function edgesMeet([first, second]: readonly [readonly [Point, Point], readonly [Point, Point]]): boolean {
  const a = turn({ edge: first, point: second[0] });
  const b = turn({ edge: first, point: second[1] });
  const c = turn({ edge: second, point: first[0] });
  const d = turn({ edge: second, point: first[1] });
  if (a * b < 0 && c * d < 0) {
    return true;
  }
  return (
    (a === 0 && within({ point: second[0], edge: first })) ||
    (b === 0 && within({ point: second[1], edge: first })) ||
    (c === 0 && within({ point: first[0], edge: second })) ||
    (d === 0 && within({ point: first[1], edge: second }))
  );
}

/**
 * The edges (by the index of the point they start at) that cross or touch an edge they don't
 * follow on from: empty when the outline goes round once. The same test as vision's
 * `crosses_itself`; it holds in photo pixels too, as the photo → paper mapping keeps lines
 * straight.
 */
export function crossingEdges(points: readonly Point[]): readonly number[] {
  const edges = edgesOf(points);
  const count = edges.length;
  const crossing = new Set<number>();
  edges.forEach((first, firstIndex) => {
    edges.slice(firstIndex + 2).forEach((second, offset) => {
      const secondIndex = firstIndex + 2 + offset;
      // The last edge follows on from the first.
      if (firstIndex === 0 && secondIndex === count - 1) {
        return;
      }
      if (edgesMeet([first, second])) {
        crossing.add(firstIndex);
        crossing.add(secondIndex);
      }
    });
  });
  return [...crossing].sort((a, b) => a - b);
}

/** The convex hull (monotone chain), counter-clockwise on the page. */
function convexHull(points: readonly Point[]): readonly Point[] {
  const sorted = [...points].sort((a, b) => a[0] - b[0] || a[1] - b[1]);
  if (sorted.length < 3) {
    return sorted;
  }
  const cross = (o: Point, a: Point, b: Point) => (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0]);
  const chain = (ordered: readonly Point[]) =>
    ordered.reduce<Point[]>((hull, point) => {
      while (hull.length >= 2 && cross(hull[hull.length - 2], hull[hull.length - 1], point) <= 0) {
        hull.pop();
      }
      hull.push(point);
      return hull;
    }, []);
  const lower = chain(sorted);
  const upper = chain([...sorted].reverse());
  return [...lower.slice(0, -1), ...upper.slice(0, -1)];
}

/**
 * The sides of the smallest rotated rectangle round the points (OpenCV's `minAreaRect`, by
 * rotating calipers on the hull), long side first.
 */
export function minAreaRect(points: readonly Point[]): { readonly long: number; readonly short: number } {
  const hull = convexHull(points);
  if (hull.length < 3) {
    return { long: 0, short: 0 };
  }
  const best = edgesOf(hull).reduce(
    (smallest, [[ax, ay], [bx, by]]) => {
      const length = Math.hypot(bx - ax, by - ay);
      if (length < EPSILON) {
        return smallest;
      }
      const [ux, uy] = [(bx - ax) / length, (by - ay) / length];
      const along = hull.map(([x, y]) => x * ux + y * uy);
      const across = hull.map(([x, y]) => -x * uy + y * ux);
      const sideA = Math.max(...along) - Math.min(...along);
      const sideB = Math.max(...across) - Math.min(...across);
      return sideA * sideB < smallest.area ? { area: sideA * sideB, sideA, sideB } : smallest;
    },
    { area: Number.POSITIVE_INFINITY, sideA: 0, sideB: 0 },
  );
  return { long: Math.max(best.sideA, best.sideB), short: Math.min(best.sideA, best.sideB) };
}

/** A circle. */
interface Circle {
  readonly center: Point;
  readonly radius: number;
}

/** How far outside a circle a point may lie and still count as in it (rounding). */
const CIRCLE_SLACK = 1e-7;

function inCircle({ circle, point }: { circle: Circle; point: Point }): boolean {
  const reach = Math.hypot(point[0] - circle.center[0], point[1] - circle.center[1]);
  return reach <= circle.radius * (1 + CIRCLE_SLACK) + CIRCLE_SLACK;
}

/** The circle with `a` and `b` at the ends of a diameter. */
function circleOfTwo([a, b]: readonly [Point, Point]): Circle {
  return { center: [(a[0] + b[0]) / 2, (a[1] + b[1]) / 2], radius: Math.hypot(b[0] - a[0], b[1] - a[1]) / 2 };
}

/** The circle through three points; for three in a line, the circle on the two farthest apart. */
function circleOfThree([a, b, c]: readonly [Point, Point, Point]): Circle {
  const d = 2 * (a[0] * (b[1] - c[1]) + b[0] * (c[1] - a[1]) + c[0] * (a[1] - b[1]));
  if (Math.abs(d) < EPSILON) {
    const pairs: readonly (readonly [Point, Point])[] = [
      [a, b],
      [b, c],
      [a, c],
    ];
    return pairs.map(circleOfTwo).reduce((widest, circle) => (circle.radius > widest.radius ? circle : widest));
  }
  const [sa, sb, sc] = [a, b, c].map(([x, y]) => x * x + y * y);
  const center: Point = [
    (sa * (b[1] - c[1]) + sb * (c[1] - a[1]) + sc * (a[1] - b[1])) / d,
    (sa * (c[0] - b[0]) + sb * (a[0] - c[0]) + sc * (b[0] - a[0])) / d,
  ];
  return { center, radius: Math.hypot(a[0] - center[0], a[1] - center[1]) };
}

/**
 * The smallest circle round the points (OpenCV's `minEnclosingCircle`, which vision sizes
 * round products by): the incremental algorithm over the hull's corners.
 */
export function enclosingCircle(points: readonly Point[]): Circle {
  const hull = convexHull(points);
  let circle: Circle = { center: hull[0] ?? [0, 0], radius: 0 };
  hull.forEach((first, i) => {
    if (inCircle({ circle, point: first })) {
      return;
    }
    circle = { center: first, radius: 0 };
    hull.slice(0, i).forEach((second, j) => {
      if (inCircle({ circle, point: second })) {
        return;
      }
      circle = circleOfTwo([first, second]);
      hull.slice(0, j).forEach((third) => {
        if (!inCircle({ circle, point: third })) {
          circle = circleOfThree([first, second, third]);
        }
      });
    });
  });
  return circle;
}

/** An outline's size from above, as the editor shows it while the points move. */
export type LiveSize =
  | { readonly shape: "round"; readonly diameterMm: number; readonly areaMm2: number }
  | { readonly shape: "box"; readonly lengthMm: number; readonly widthMm: number; readonly areaMm2: number };

/**
 * The outline's size in mm (the points in mm on the sheet), as vision measures an edited
 * outline: round when it is circular and about as long as it is wide (sized by the smallest
 * circle round it), else the rotated box round it.
 */
export function liveSize(pointsMm: readonly Point[]): LiveSize {
  const areaMm2 = polygonArea(pointsMm);
  const { long, short } = minAreaRect(pointsMm);
  const length = perimeter(pointsMm);
  const circularity = length > 0 ? (4 * Math.PI * areaMm2) / (length * length) : 0;
  const aspect = short > 0 ? long / short : Number.POSITIVE_INFINITY;
  return circularity >= ROUND_MIN_CIRCULARITY && aspect <= ROUND_MAX_ASPECT
    ? { shape: "round", diameterMm: 2 * enclosingCircle(pointsMm).radius, areaMm2 }
    : { shape: "box", lengthMm: long, widthMm: short, areaMm2 };
}

/** The nearest spot on a segment to a point, and how far along it lies (0 to 1). */
export function nearestOnSegment({ point, segment: [start, end] }: { point: Point; segment: readonly [Point, Point] }): {
  readonly at: Point;
  readonly along: number;
  readonly distance: number;
} {
  const [dx, dy] = [end[0] - start[0], end[1] - start[1]];
  const lengthSquared = dx * dx + dy * dy;
  const along =
    lengthSquared < EPSILON
      ? 0
      : Math.min(Math.max(((point[0] - start[0]) * dx + (point[1] - start[1]) * dy) / lengthSquared, 0), 1);
  const at: Point = [start[0] + along * dx, start[1] + along * dy];
  return { at, along, distance: Math.hypot(point[0] - at[0], point[1] - at[1]) };
}

/** What an outline point or edge is under the pointer. */
export type OutlineHit =
  | { readonly kind: "point"; readonly index: number }
  /** A new point at `at`, put in after point `after`. */
  | { readonly kind: "edge"; readonly after: number; readonly at: Point }
  | { readonly kind: "nothing" };

/**
 * The point within `pointReach` of `pointer`, nearest first; else the nearest spot on an edge
 * within `edgeReach` (away from the edge's ends, which are points); else nothing. Reaches are
 * in the same units as the points.
 */
export function hitOutline({
  points,
  pointer,
  pointReach,
  edgeReach,
}: {
  points: readonly Point[];
  pointer: Point;
  pointReach: number;
  edgeReach: number;
}): OutlineHit {
  const nearestPoint = points.reduce(
    (best, [x, y], index) => {
      const distance = Math.hypot(x - pointer[0], y - pointer[1]);
      return distance < best.distance ? { index, distance } : best;
    },
    { index: -1, distance: Number.POSITIVE_INFINITY },
  );
  if (nearestPoint.index >= 0 && nearestPoint.distance <= pointReach) {
    return { kind: "point", index: nearestPoint.index };
  }
  const nearestEdge = edgesOf(points).reduce(
    (best, segment, index) => {
      const spot = nearestOnSegment({ point: pointer, segment });
      return spot.distance < best.distance ? { after: index, at: spot.at, distance: spot.distance } : best;
    },
    { after: -1, at: pointer, distance: Number.POSITIVE_INFINITY },
  );
  return nearestEdge.after >= 0 && nearestEdge.distance <= edgeReach
    ? { kind: "edge", after: nearestEdge.after, at: nearestEdge.at }
    : { kind: "nothing" };
}

/** The SVG path of a closed outline. */
export function closedPath(points: readonly Point[]): string {
  return points.length === 0 ? "" : `M${points.map(([x, y]) => `${x.toFixed(1)} ${y.toFixed(1)}`).join("L")}Z`;
}
