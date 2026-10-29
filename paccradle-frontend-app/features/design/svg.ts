import type { Path, Point, Segment } from "./types";

/**
 * SVG path data for the engine's paths, in mm. Arcs stay arcs; one longer than a half
 * turn is drawn in two halves, since a single SVG arc can't draw a full circle.
 */

/** Half a turn: the longest arc one SVG arc command draws reliably. */
const HALF_TURN_DEG = 180;

/** The `d` attribute for a path. */
export function pathData(path: Path): string {
  const commands = [`M${path.start[0]} ${path.start[1]}`];
  let current = path.start;
  for (const segment of path.segments) {
    const drawn = segmentData({ from: current, segment });
    commands.push(drawn.data);
    current = drawn.end;
  }
  if (path.closed) {
    commands.push("Z");
  }
  return commands.join(" ");
}

/** One segment's commands and where it ends. */
function segmentData({ from, segment }: { readonly from: Point; readonly segment: Segment }): {
  readonly data: string;
  readonly end: Point;
} {
  if (segment.kind === "line") {
    return { data: `L${segment.to[0]} ${segment.to[1]}`, end: segment.to };
  }
  const parts = Math.abs(segment.sweep_deg) > HALF_TURN_DEG ? 2 : 1;
  const partDeg = segment.sweep_deg / parts;
  const radius = Math.hypot(from[0] - segment.center[0], from[1] - segment.center[1]);
  // Positive sweeps turn clockwise on screen (y down), which is SVG's sweep flag 1.
  const sweepFlag = partDeg > 0 ? 1 : 0;
  const ends = Array.from({ length: parts }, (_, index) =>
    turn({ point: from, center: segment.center, deg: partDeg * (index + 1) }),
  );
  const data = ends
    .map((end) => `A${radius} ${radius} 0 0 ${sweepFlag} ${end[0]} ${end[1]}`)
    .join(" ");
  return { data, end: ends[ends.length - 1] };
}

/** `point` turned by `deg` about `center`, clockwise on screen when positive. */
function turn({
  point,
  center,
  deg,
}: {
  readonly point: Point;
  readonly center: Point;
  readonly deg: number;
}): Point {
  const rad = (deg * Math.PI) / 180;
  const [dx, dy] = [point[0] - center[0], point[1] - center[1]];
  return [
    center[0] + dx * Math.cos(rad) - dy * Math.sin(rad),
    center[1] + dx * Math.sin(rad) + dy * Math.cos(rad),
  ];
}
