import type { ReactNode } from "react";
import type { Board } from "../types";
import styles from "./ChoiceArt.module.css";

/**
 * Line drawings for the design choices (M1-E4): each insert style and box as a small
 * isometric model (board in cream, products in sage, the box in kraft), and each board's
 * cross-section drawn to scale. Lines draw themselves in; decorative only.
 */

/** The drawings' box (viewBox units). */
const WIDTH = 120;
const HEIGHT = 90;
/** Isometric axes: 30° from the horizontal. */
const COS30 = Math.cos(Math.PI / 6);
const SIN30 = 0.5;
/** Points round a drawn circle. */
const CIRCLE_STEPS = 28;

type Vec3 = readonly [number, number, number];

/** Where a point of the model lands in the drawing: x to the right-down, y to the left-down, z up. */
function project({ point, origin }: { point: Vec3; origin: readonly [number, number] }): string {
  const [x, y, z] = point;
  const sx = origin[0] + (x - y) * COS30;
  const sy = origin[1] + (x + y) * SIN30 - z;
  return `${sx.toFixed(2)} ${sy.toFixed(2)}`;
}

/** A closed polygon through model points. */
function face({ points, origin }: { points: readonly Vec3[]; origin: readonly [number, number] }): string {
  return `M${points.map((point) => project({ point, origin })).join("L")}Z`;
}

/** A level circle of the model, as a closed path. */
function ring({ center, radius, origin }: { center: Vec3; radius: number; origin: readonly [number, number] }): string {
  const points = Array.from({ length: CIRCLE_STEPS }, (_, step): Vec3 => {
    const angle = (step * 2 * Math.PI) / CIRCLE_STEPS;
    return [center[0] + radius * Math.cos(angle), center[1] + radius * Math.sin(angle), center[2]];
  });
  return face({ points, origin });
}

/** A standing cylinder (a jar or bottle): its side, then its top. */
function Jar({ at, radius, height, origin, delay }: { at: Vec3; radius: number; height: number; origin: readonly [number, number]; delay: number }) {
  const d = radius / Math.SQRT2;
  const side = face({
    points: [
      [at[0] - d, at[1] + d, at[2]],
      [at[0] + d, at[1] - d, at[2]],
      [at[0] + d, at[1] - d, at[2] + height],
      [at[0] - d, at[1] + d, at[2] + height],
    ],
    origin,
  });
  return (
    <g className={styles.jar} style={{ animationDelay: `${delay}ms` }}>
      <path className={styles.jarSide} d={side} />
      <path className={styles.jarBase} d={ring({ center: at, radius, origin })} />
      <path className={styles.jarTop} d={ring({ center: [at[0], at[1], at[2] + height], radius, origin })} />
    </g>
  );
}

/** A slab of board: its top face with holes, and its two front edges. */
function Slab({ size, z, thickness, holes, origin }: { size: readonly [number, number]; z: number; thickness: number; holes: ReadonlyArray<readonly [number, number, number]>; origin: readonly [number, number] }) {
  const [l, w] = size;
  const top = face({ points: [[0, 0, z], [l, 0, z], [l, w, z], [0, w, z]], origin });
  const cut = holes.map(([x, y, r]) => ring({ center: [x, y, z], radius: r, origin })).join("");
  return (
    <>
      <path className={styles.edge} d={face({ points: [[0, w, z], [l, w, z], [l, w, z - thickness], [0, w, z - thickness]], origin })} />
      <path className={styles.edge} d={face({ points: [[l, 0, z], [l, w, z], [l, w, z - thickness], [l, 0, z - thickness]], origin })} />
      <path className={styles.board} d={`${top}${cut}`} pathLength={1} />
    </>
  );
}

const GRID_ORIGIN = [52, 26] as const;
const CARD_ORIGIN = [50, 30] as const;
const CRADLE_ORIGIN = [50, 22] as const;

/** Partition grid: two long and four short strips crossing, jars in the cells. */
function GridArt() {
  const [l, w, h] = [60, 40, 16];
  const strips: ReadonlyArray<{ readonly key: string; readonly points: readonly Vec3[]; readonly depth: number }> = [
    ...[0, 20, 40].map((y) => ({ key: `y${y}`, points: [[0, y, 0], [l, y, 0], [l, y, h], [0, y, h]] as const, depth: y + l / 2 })),
    ...[0, 20, 40, 60].map((x) => ({ key: `x${x}`, points: [[x, 0, 0], [x, w, 0], [x, w, h], [x, 0, h]] as const, depth: x + w / 2 })),
  ];
  const jars = [10, 30, 50].flatMap((x) => [10, 30].map((y): Vec3 => [x, y, 0]));
  const sorted = [...strips].sort((a, b) => a.depth - b.depth);
  return (
    <>
      <path className={styles.floor} d={face({ points: [[0, 0, 0], [l, 0, 0], [l, w, 0], [0, w, 0]], origin: GRID_ORIGIN })} />
      {jars.map((at, index) => (
        <Jar key={`${at[0]}-${at[1]}`} at={at} radius={6.5} height={24} origin={GRID_ORIGIN} delay={500 + index * 60} />
      ))}
      {sorted.map((strip) => (
        <path key={strip.key} className={styles.strip} d={face({ points: strip.points, origin: GRID_ORIGIN })} pathLength={1} />
      ))}
    </>
  );
}

/** Cutout card: a flat card with round holes, one jar sitting in. */
function CardArt() {
  const holes = [
    [14, 13, 7],
    [34, 13, 7],
    [54, 13, 7],
    [24, 31, 7],
    [44, 31, 7],
  ] as const;
  return (
    <>
      <Slab size={[68, 44]} z={0} thickness={3} holes={holes} origin={CARD_ORIGIN} />
      <Jar at={[44, 31, -2]} radius={6} height={12} origin={CARD_ORIGIN} delay={600} />
      <Jar at={[24, 31, -2]} radius={6} height={12} origin={CARD_ORIGIN} delay={680} />
    </>
  );
}

/** Cradle: a raised deck on folded legs, bottles standing through it. */
function CradleArt() {
  const [l, w, deck] = [64, 40, 16];
  const legs = [3, w - 3].map((y) =>
    face({ points: [[0, y, 0], [l, y, 0], [l, y, deck], [0, y, deck]], origin: CRADLE_ORIGIN }),
  );
  return (
    <>
      <path className={styles.floor} d={face({ points: [[0, 0, 0], [l, 0, 0], [l, w, 0], [0, w, 0]], origin: CRADLE_ORIGIN })} />
      <path className={styles.strip} d={legs[0]} pathLength={1} />
      <Slab size={[l, w]} z={deck} thickness={2} holes={[[20, 20, 8], [44, 20, 8]]} origin={CRADLE_ORIGIN} />
      <path className={styles.strip} d={legs[1]} pathLength={1} />
      <Jar at={[20, 20, 2]} radius={7} height={34} origin={CRADLE_ORIGIN} delay={600} />
      <Jar at={[44, 20, 2]} radius={7} height={34} origin={CRADLE_ORIGIN} delay={700} />
    </>
  );
}

const BOX_ORIGIN = [52, 47] as const;
const BOX: Vec3 = [48, 32, 16];
/** How far the mailer's lid stands open from closed. */
const LID_OPEN_DEG = 80;

/** The open box's floor, back and side walls, and front walls (drawn last). */
function BoxBody({ origin }: { origin: readonly [number, number] }) {
  const [l, w, h] = BOX;
  return (
    <>
      <path className={styles.kraft} d={face({ points: [[0, 0, 0], [l, 0, 0], [l, 0, h], [0, 0, h]], origin })} />
      <path className={styles.kraft} d={face({ points: [[0, 0, 0], [0, w, 0], [0, w, h], [0, 0, h]], origin })} />
      <path className={styles.floor} d={face({ points: [[0, 0, 0], [l, 0, 0], [l, w, 0], [0, w, 0]], origin })} />
      <Jar at={[14, 16, 0]} radius={6} height={12} origin={origin} delay={650} />
      <Jar at={[34, 16, 0]} radius={6} height={12} origin={origin} delay={730} />
      <path className={styles.kraftFront} d={face({ points: [[0, w, 0], [l, w, 0], [l, w, h], [0, w, h]], origin })} pathLength={1} />
      <path className={styles.kraftFront} d={face({ points: [[l, 0, 0], [l, w, 0], [l, w, h], [l, 0, h]], origin })} pathLength={1} />
    </>
  );
}

/** Mailer box (FEFCO 0427 style): the lid, as deep as the box, open on its hinge at the back. */
function MailerArt() {
  const [l, w, h] = BOX;
  const open = (LID_OPEN_DEG * Math.PI) / 180;
  const [lidY, lidZ] = [w * Math.cos(open), h + w * Math.sin(open)];
  return (
    <>
      <path
        className={styles.lid}
        d={face({ points: [[0, 0, h], [l, 0, h], [l, lidY, lidZ], [0, lidY, lidZ]], origin: BOX_ORIGIN })}
        pathLength={1}
      />
      <BoxBody origin={BOX_ORIGIN} />
    </>
  );
}

/** Shipping box (FEFCO 0201): four flaps standing open. */
function ShippingArt() {
  const [l, w, h] = BOX;
  const flap = 12;
  return (
    <>
      <path className={styles.lid} d={face({ points: [[0, 0, h], [l, 0, h], [l, -4, h + flap], [0, -4, h + flap]], origin: BOX_ORIGIN })} pathLength={1} />
      <path className={styles.lid} d={face({ points: [[0, 0, h], [0, w, h], [-4, w, h + flap], [-4, 0, h + flap]], origin: BOX_ORIGIN })} pathLength={1} />
      <BoxBody origin={BOX_ORIGIN} />
      <path className={styles.lid} d={face({ points: [[0, w, h], [l, w, h], [l, w + 4, h + flap], [0, w + 4, h + flap]], origin: BOX_ORIGIN })} pathLength={1} />
      <path className={styles.lid} d={face({ points: [[l, 0, h], [l, w, h], [l + 4, w, h + flap], [l + 4, 0, h + flap]], origin: BOX_ORIGIN })} pathLength={1} />
    </>
  );
}

/** The insert alone, in a dashed box of the person's own. */
function NoBoxArt() {
  const [l, w, h] = BOX;
  const ghost = [
    face({ points: [[0, w, 0], [l, w, 0], [l, w, h], [0, w, h]], origin: BOX_ORIGIN }),
    face({ points: [[l, 0, 0], [l, w, 0], [l, w, h], [l, 0, h]], origin: BOX_ORIGIN }),
    face({ points: [[0, 0, h], [l, 0, h], [l, w, h], [0, w, h]], origin: BOX_ORIGIN }),
  ].join("");
  return (
    <>
      <Slab size={[l, w]} z={4} thickness={2} holes={[[14, 16, 6.5], [34, 16, 6.5]]} origin={BOX_ORIGIN} />
      <Jar at={[14, 16, 2]} radius={6} height={12} origin={BOX_ORIGIN} delay={600} />
      <Jar at={[34, 16, 2]} radius={6} height={12} origin={BOX_ORIGIN} delay={680} />
      <path className={styles.ghost} d={ghost} />
    </>
  );
}

const ART = {
  partition_grid: <GridArt />,
  cutout_card: <CardArt />,
  cradle_platform: <CradleArt />,
  mailer: <MailerArt />,
  shipping: <ShippingArt />,
  none: <NoBoxArt />,
} as const satisfies Readonly<Record<string, ReactNode>>;

/** Names of the drawings. */
export type ArtName = keyof typeof ART;

/** A small isometric drawing of an insert style or a box. */
export function ChoiceArt({ name, className = "" }: { readonly name: ArtName; readonly className?: string }) {
  return (
    <svg viewBox={`0 0 ${WIDTH} ${HEIGHT}`} aria-hidden="true" className={`${styles.art} ${className}`}>
      {ART[name]}
    </svg>
  );
}

// ---------------------------------------------------------------- board cross-sections

/** Cross-section scale (units per mm) and length. */
const SECTION_SCALE = 5;
const SECTION_LENGTH = 112;
const SECTION_HEIGHT = 28;
/** Flute pitch per corrugated board, in mm (common values: E ≈ 3.3, B ≈ 6.5, C ≈ 8). */
const FLUTE_PITCH_MM: Readonly<Partial<Record<Board, number>>> = { e_flute: 3.3, b_flute: 6.5, c_flute: 8 };
/** A liner's drawn thickness, in mm. */
const LINER_MM = 0.25;

/** The flute's wave between two liners, from x 0 to `SECTION_LENGTH`. */
function fluteWave({ pitch, top, bottom }: { pitch: number; top: number; bottom: number }): string {
  const half = (pitch * SECTION_SCALE) / 2;
  const steps = Math.ceil(SECTION_LENGTH / half);
  const middle = (top + bottom) / 2;
  const commands = Array.from({ length: steps }, (_, step) => {
    const x = (step + 1) * half;
    const peak = step % 2 === 0 ? top : bottom;
    return `Q${(x - half / 2).toFixed(2)} ${peak.toFixed(2)} ${x.toFixed(2)} ${middle.toFixed(2)}`;
  });
  return `M0 ${middle.toFixed(2)}${commands.join("")}`;
}

/** A board's edge, cut through, to scale: solid for paperboard and chipboard, liners and flutes for corrugated. */
export function BoardSection({ board, thicknessMm }: { readonly board: Board; readonly thicknessMm: number }) {
  const drawn = Math.min(thicknessMm * SECTION_SCALE, SECTION_HEIGHT - 4);
  const top = (SECTION_HEIGHT - drawn) / 2;
  const bottom = top + drawn;
  const pitch = FLUTE_PITCH_MM[board];
  const liner = Math.max(LINER_MM * SECTION_SCALE, 1);
  return (
    <svg viewBox={`0 0 ${SECTION_LENGTH} ${SECTION_HEIGHT}`} aria-hidden="true" className={styles.section} data-board={board}>
      {pitch === undefined ? (
        <rect className={styles.solid} x={0} y={top} width={SECTION_LENGTH} height={drawn} rx={0.6} />
      ) : (
        <>
          <path className={styles.flute} d={fluteWave({ pitch, top: top + liner, bottom: bottom - liner })} pathLength={1} />
          <rect className={styles.liner} x={0} y={top} width={SECTION_LENGTH} height={liner} />
          <rect className={styles.liner} x={0} y={bottom - liner} width={SECTION_LENGTH} height={liner} />
        </>
      )}
    </svg>
  );
}
