import type { Board, Design, InsertSpec, Material, Point, Polygon } from "./types";

/**
 * Starting designs for checks and demos: one round product in any insert style, in a box
 * sized round it. Board numbers are the engine's starting values (CLAUDE.md §6.4,
 * `Material::starting_values`); keep the two in step.
 */

/** The engine's starting values per board: thickness, clearance and slot tolerance (mm). */
export const STARTING_MATERIALS: Readonly<Record<Board, Material>> = {
  paperboard: { board: "paperboard", thickness_mm: 0.5, clearance_mm: 1.0, slot_tolerance_mm: 0.3 },
  chipboard: { board: "chipboard", thickness_mm: 1.5, clearance_mm: 1.0, slot_tolerance_mm: 0.3 },
  e_flute: { board: "e_flute", thickness_mm: 1.6, clearance_mm: 1.5, slot_tolerance_mm: 0.5 },
  b_flute: { board: "b_flute", thickness_mm: 3.0, clearance_mm: 1.75, slot_tolerance_mm: 0.5 },
  c_flute: { board: "c_flute", thickness_mm: 4.0, clearance_mm: 2.0, slot_tolerance_mm: 0.5 },
};

/** The insert styles, in the order the editor offers them. */
export const InsertStyle = {
  Grid: "partition_grid",
  Card: "cutout_card",
  Cradle: "cradle_platform",
} as const;

export type InsertStyle = (typeof InsertStyle)[keyof typeof InsertStyle];

/** Points round a sample product's outline: smooth enough, like a vision outline. */
const CIRCLE_POINTS = 64;
/** The sample product: a jar this tall, six of them. */
const SAMPLE_HEIGHT_MM = 85;
const SAMPLE_QUANTITY = 6;
const GRID = { rows: 2, columns: 3, dividerShare: 0.7 } as const;
/** A card's and a deck's margin: at least a web of E-flute (3 × 1.6 mm), rounded up. */
const MARGIN_MM = 10;
/** The cradle deck stands at this share of the product's height. */
const DECK_SHARE = 0.4;

/** A circle of `diameterMm` inside its bounding box from the origin. */
export function circle(diameterMm: number): Polygon {
  const radius = diameterMm / 2;
  return Array.from({ length: CIRCLE_POINTS }, (_, step): Point => {
    const angle = (step * 2 * Math.PI) / CIRCLE_POINTS;
    return [radius + radius * Math.cos(angle), radius + radius * Math.sin(angle)];
  });
}

/** The settings each style starts with for the sample jar. */
function insertSpec(style: InsertStyle): InsertSpec {
  switch (style) {
    case InsertStyle.Grid:
      return {
        style,
        rows: GRID.rows,
        columns: GRID.columns,
        divider_height_mm: SAMPLE_HEIGHT_MM * GRID.dividerShare,
        outer_frame: false,
        finger_notches: true,
      };
    case InsertStyle.Card:
      return {
        style,
        margin_mm: MARGIN_MM,
        finger_notches: { side: "bottom", along: 0.5 },
        align: "center",
        placements: [],
      };
    case InsertStyle.Cradle:
      return {
        style,
        deck_height_mm: SAMPLE_HEIGHT_MM * DECK_SHARE,
        margin_mm: MARGIN_MM,
        align: "center",
        placements: [],
      };
  }
}

/** Six round jars of `diameterMm` in E-flute, held by `style`, in a B-flute shipping box. */
export function sampleDesign({
  style,
  diameterMm,
}: {
  readonly style: InsertStyle;
  readonly diameterMm: number;
}): Design {
  return {
    material: STARTING_MATERIALS.e_flute,
    products: [
      {
        id: "jar",
        contour_mm: circle(diameterMm),
        height_mm: SAMPLE_HEIGHT_MM,
        quantity: SAMPLE_QUANTITY,
      },
    ],
    insert: insertSpec(style),
    outer_box: { style: "shipping", material: STARTING_MATERIALS.b_flute, inside_mm: null },
  };
}
