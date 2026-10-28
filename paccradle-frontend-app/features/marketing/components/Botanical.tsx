import type { CSSProperties } from "react";
import styles from "./Botanical.module.css";

type Point = readonly [number, number];

/** Stem as a cubic Bézier curve in a 240 × 400 box. */
const STEM: { readonly start: Point; readonly c1: Point; readonly c2: Point; readonly end: Point } = {
  start: [44, 392],
  c1: [70, 270],
  c2: [150, 160],
  end: [196, 16],
};
const LEAF_COUNT = 11;
const FIRST_LEAF_T = 0.1;
const LAST_LEAF_T = 0.98;
/** How far a leaf sits from the stem, its half length and half width. */
const LEAF_OFFSET = 18;
const LEAF_HALF_LENGTH = 17;
const LEAF_HALF_WIDTH = 9;
/** Leaves point up the stem at this angle. */
const LEAF_TILT_DEG = 38;
/** Leaves get smaller towards the tip. */
const TIP_SCALE = 0.55;
const RADIANS_TO_DEGREES = 180 / Math.PI;
const ROUNDING = 100;

/** A lens-shaped leaf around (0, 0), pointing along the x axis. */
const LEAF_PATH = `M${-LEAF_HALF_LENGTH} 0Q0 ${-LEAF_HALF_WIDTH * 2} ${LEAF_HALF_LENGTH} 0Q0 ${LEAF_HALF_WIDTH * 2} ${-LEAF_HALF_LENGTH} 0Z`;
const MIDRIB_PATH = `M${-LEAF_HALF_LENGTH * 0.8} 0L${LEAF_HALF_LENGTH * 0.8} 0`;

function round(value: number): number {
  return Math.round(value * ROUNDING) / ROUNDING;
}

/** Point on the stem at `t` (0 = bottom, 1 = tip). */
function stemPoint(t: number): Point {
  const u = 1 - t;
  const axis = (index: 0 | 1): number =>
    u ** 3 * STEM.start[index] +
    3 * u ** 2 * t * STEM.c1[index] +
    3 * u * t ** 2 * STEM.c2[index] +
    t ** 3 * STEM.end[index];
  return [axis(0), axis(1)];
}

/** Direction of the stem at `t`. */
function stemTangent(t: number): Point {
  const u = 1 - t;
  const axis = (index: 0 | 1): number =>
    3 * u ** 2 * (STEM.c1[index] - STEM.start[index]) +
    6 * u * t * (STEM.c2[index] - STEM.c1[index]) +
    3 * t ** 2 * (STEM.end[index] - STEM.c2[index]);
  return [axis(0), axis(1)];
}

interface Leaf {
  readonly base: Point;
  readonly center: Point;
  readonly angle: number;
  readonly scale: number;
}

/** Leaves alternate left and right along the stem. */
function leaves(): readonly Leaf[] {
  return Array.from({ length: LEAF_COUNT }, (_, index) => {
    const share = index / (LEAF_COUNT - 1);
    const t = FIRST_LEAF_T + (LAST_LEAF_T - FIRST_LEAF_T) * share;
    const [x, y] = stemPoint(t);
    const [dx, dy] = stemTangent(t);
    const length = Math.hypot(dx, dy);
    const side = index % 2 === 0 ? 1 : -1;
    const scale = 1 - (1 - TIP_SCALE) * share;
    const offset = LEAF_OFFSET * scale;
    return {
      base: [round(x), round(y)],
      center: [round(x - (dy / length) * side * offset), round(y + (dx / length) * side * offset)],
      angle: round(Math.atan2(dy, dx) * RADIANS_TO_DEGREES - side * LEAF_TILT_DEG),
      scale: round(scale),
    };
  });
}

/** Props for the botanical line drawing. */
export interface BotanicalProps {
  readonly className?: string;
}

/**
 * A eucalyptus sprig in thin line art (decorative). The stem and leaves draw themselves in
 * when it scrolls into view, then it sways gently.
 */
export function Botanical({ className = "" }: BotanicalProps) {
  return (
    <svg
      viewBox="0 0 240 400"
      aria-hidden="true"
      className={`${styles.sprig} ${className}`}
      data-reveal="draw"
    >
      <path
        className={styles.stem}
        pathLength={1}
        d={`M${STEM.start} C${STEM.c1} ${STEM.c2} ${STEM.end}`}
      />
      {leaves().map((leaf, index) => (
        <g key={`${leaf.base}`} style={{ "--leaf-index": index } as CSSProperties}>
          <path
            className={styles.stalk}
            pathLength={1}
            d={`M${leaf.base[0]} ${leaf.base[1]}L${leaf.center[0]} ${leaf.center[1]}`}
          />
          <g
            transform={`translate(${leaf.center[0]} ${leaf.center[1]}) rotate(${leaf.angle}) scale(${leaf.scale})`}
          >
            <path className={styles.leaf} pathLength={1} d={LEAF_PATH} />
            <path className={styles.midrib} pathLength={1} d={MIDRIB_PATH} />
          </g>
        </g>
      ))}
    </svg>
  );
}
