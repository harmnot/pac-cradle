import { none, some, type Maybe } from "@/lib/maybe";
import { circle, InsertStyle, STARTING_MATERIALS } from "./samples";
import type {
  Board,
  BoxStyle,
  Design,
  InsertSpec,
  Material,
  Packaging,
  Polygon,
} from "./types";

/**
 * The design choices (CLAUDE.md §3 steps 5–8, MILESTONES M1-E4): the product's height and
 * how many, the insert style, the board and the outer box, turned into the engine's
 * `Design`. Pure functions: the page keeps the choices, this module decides the defaults
 * and builds the spec the WASM preview and `POST /designs` both take.
 */

// ---------------------------------------------------------------- the product

/** Where the product's sizes came from. */
export const ProductSource = {
  /** Measured from a photo (vision's outline). */
  Measured: "measured",
  /** An outline the person drew round the photo by hand (M1-E3). */
  Drawn: "drawn",
  /** Typed from a ruler (CLAUDE.md §3 step 1c). */
  Typed: "typed",
} as const;

export type ProductSource = (typeof ProductSource)[keyof typeof ProductSource];

/** The product the insert is designed round. */
export interface ChosenProduct {
  readonly source: ProductSource;
  /** Top view in mm from the outline's top-left corner, as vision gives it. */
  readonly contourMm: Polygon;
  readonly round: boolean;
  /** Its size from above: the diameter when round, else length × width (length the longer). */
  readonly lengthMm: number;
  readonly widthMm: number;
  /** Its height, when known: typed, or added to the capture earlier. */
  readonly heightMm: Maybe<number>;
}

/** A measured product from a capture's result (`result.products[]`, CLAUDE.md §5.5). */
export interface MeasuredProduct {
  readonly shape: "round" | "rectangular" | "freeform";
  readonly bbox_mm: { readonly l: number; readonly w: number };
  readonly diameter_mm: number | null;
  readonly contour_mm: Polygon;
  readonly contour_source: "detected" | "user";
  readonly height_mm: number | null;
}

/** The product a capture measured. */
export function measuredProduct(product: MeasuredProduct): ChosenProduct {
  const round = product.shape === "round" && product.diameter_mm !== null;
  const diameter = product.diameter_mm ?? 0;
  return {
    source: product.contour_source === "user" ? ProductSource.Drawn : ProductSource.Measured,
    contourMm: product.contour_mm,
    round,
    lengthMm: round ? diameter : Math.max(product.bbox_mm.l, product.bbox_mm.w),
    widthMm: round ? diameter : Math.min(product.bbox_mm.l, product.bbox_mm.w),
    heightMm: product.height_mm === null ? none : some(product.height_mm),
  };
}

/** Sizes typed by hand (`features/capture/typed-product.ts`). */
export type TypedSizes =
  | { readonly shape: "round"; readonly diameterMm: number; readonly heightMm: number }
  | {
      readonly shape: "rectangular";
      readonly lengthMm: number;
      readonly widthMm: number;
      readonly heightMm: number;
    };

/** A typed product: a circle or a rectangle from its sizes. */
export function typedProduct(sizes: TypedSizes): ChosenProduct {
  if (sizes.shape === "round") {
    return {
      source: ProductSource.Typed,
      contourMm: circle(sizes.diameterMm),
      round: true,
      lengthMm: sizes.diameterMm,
      widthMm: sizes.diameterMm,
      heightMm: some(sizes.heightMm),
    };
  }
  const [length, width] = [sizes.lengthMm, sizes.widthMm];
  return {
    source: ProductSource.Typed,
    contourMm: [
      [0, 0],
      [length, 0],
      [length, width],
      [0, width],
    ],
    round: false,
    lengthMm: length,
    widthMm: width,
    heightMm: some(sizes.heightMm),
  };
}

/** Heights the app takes, as vision checks them (`HEIGHT_RANGE_MM`: 0.1–1000 mm). */
export const HEIGHT_MIN_MM = 0.1;
export const HEIGHT_MAX_MM = 1000;

/** The height in a field's text, when it's a height the app takes. */
export function parseHeight(text: string): Maybe<number> {
  const value = Number.parseFloat(text.trim().replace(",", "."));
  return Number.isFinite(value) && value >= HEIGHT_MIN_MM && value <= HEIGHT_MAX_MM
    ? some(value)
    : none;
}

// ---------------------------------------------------------------- quantity

/** Copies of the product one insert holds, as the page offers them (the engine takes 1000). */
export const QUANTITY_MIN = 1;
export const QUANTITY_MAX = 100;
/** Counts offered as one tap. */
export const QUANTITY_PRESETS = [1, 2, 4, 6, 12, 24] as const;

/** `value` kept within the quantities on offer, as a whole number. */
export function clampQuantity(value: number): number {
  if (!Number.isFinite(value)) {
    return QUANTITY_MIN;
  }
  return Math.min(QUANTITY_MAX, Math.max(QUANTITY_MIN, Math.round(value)));
}

// ---------------------------------------------------------------- partition grid layout

/** Cells across (rows, along y) and along (columns, along x: the box's length). */
export interface GridLayout {
  readonly rows: number;
  readonly columns: number;
}

/** A single row is offered up to this many cells; more rows keep within 3 columns per row. */
const GRID_MAX_SINGLE_ROW = 6;
const GRID_MAX_COLUMNS_PER_ROW = 3;
/** Layouts offered at most, the squarest ones. */
const GRID_MAX_OFFERED = 4;

/**
 * The grids that hold `quantity` with no empty row: fewest rows first, each with as few
 * columns as fit, and none much longer than wide. Rows never outnumber columns (the same
 * grid turned is the same box turned).
 */
export function gridLayouts(quantity: number): readonly GridLayout[] {
  const count = clampQuantity(quantity);
  const most = Math.floor(Math.sqrt(count));
  const layouts = Array.from({ length: most }, (_, index): GridLayout => {
    const rows = index + 1;
    return { rows, columns: Math.ceil(count / rows) };
  }).filter(
    ({ rows, columns }) =>
      columns <= GRID_MAX_SINGLE_ROW || columns <= GRID_MAX_COLUMNS_PER_ROW * rows,
  );
  const distinct = layouts.filter(
    (layout, index) => layouts.findIndex((other) => sameLayout(other, layout)) === index,
  );
  return distinct.slice(-GRID_MAX_OFFERED);
}

/** The grid to start with: the fewest empty cells, then the squarest. */
export function defaultGridLayout(quantity: number): GridLayout {
  const count = clampQuantity(quantity);
  const empty = (layout: GridLayout) => layout.rows * layout.columns - count;
  const offered = gridLayouts(count);
  return offered.reduce(
    (best, layout) =>
      empty(layout) < empty(best) || (empty(layout) === empty(best) && layout.rows > best.rows)
        ? layout
        : best,
    offered[0] ?? { rows: 1, columns: count },
  );
}

/** Whether two layouts are the same grid. */
export function sameLayout(a: GridLayout, b: GridLayout): boolean {
  return a.rows === b.rows && a.columns === b.columns;
}

/** The layout chosen if it still offers room for `quantity`, else the default. */
export function gridFor({ chosen, quantity }: { chosen: Maybe<GridLayout>; quantity: number }): GridLayout {
  if (chosen.kind === "none") {
    return defaultGridLayout(quantity);
  }
  const offered = gridLayouts(quantity).some((layout) => sameLayout(layout, chosen.value));
  return offered ? chosen.value : defaultGridLayout(quantity);
}

// ---------------------------------------------------------------- recommendations

/** Why a style is recommended (translated as `choices.style.why.<reason>`). */
export const StyleReason = {
  /** Low, flat products sit best in a card's holes. */
  Low: "low",
  /** Several identical items each get their own cell. */
  Many: "many",
  /** One to a few tall items stand on a raised deck. */
  Tall: "tall",
} as const;

export type StyleReason = (typeof StyleReason)[keyof typeof StyleReason];

/** Products no taller than this are "low" (lipsticks, jewellery, tins). */
const LOW_PRODUCT_MAX_MM = 40;
/** From this many copies of a taller product, a grid is the usual insert. */
const GRID_FROM_QUANTITY = 4;

/**
 * The style to suggest (§6.2's "best for"), with its reason. A rule of thumb until the AI
 * advisor (U12, v1) can look at the photo.
 */
export function recommendStyle({
  heightMm,
  quantity,
}: {
  heightMm: number;
  quantity: number;
}): { readonly style: InsertStyle; readonly reason: StyleReason } {
  if (heightMm <= LOW_PRODUCT_MAX_MM) {
    return { style: InsertStyle.Card, reason: StyleReason.Low };
  }
  if (quantity >= GRID_FROM_QUANTITY) {
    return { style: InsertStyle.Grid, reason: StyleReason.Many };
  }
  return { style: InsertStyle.Cradle, reason: StyleReason.Tall };
}

/** The board each style is usually cut from. */
const STYLE_BOARD: Readonly<Record<InsertStyle, Board>> = {
  [InsertStyle.Grid]: "e_flute",
  [InsertStyle.Card]: "chipboard",
  [InsertStyle.Cradle]: "e_flute",
};

/** The board to suggest for a style: chipboard for a rigid card, E-flute where board folds or slots. */
export function recommendBoard(style: InsertStyle): Board {
  return STYLE_BOARD[style];
}

/** Outer box choices: the engine's two styles, or the insert alone. */
export const BoxChoice = {
  Mailer: "mailer",
  Shipping: "shipping",
  None: "none",
} as const;

export type BoxChoice = (typeof BoxChoice)[keyof typeof BoxChoice];

/** The board a box style usually comes in. */
const BOX_BOARD: Readonly<Record<BoxStyle, Board>> = {
  mailer: "e_flute",
  shipping: "b_flute",
};

/** The board to suggest for a box: E-flute for a mailer, B-flute for a shipping box. */
export function recommendBoxBoard(style: BoxStyle): Board {
  return BOX_BOARD[style];
}

// ---------------------------------------------------------------- board

/** Every board, thinnest first, as the page lists them. */
export const BOARDS: readonly Board[] = ["paperboard", "chipboard", "e_flute", "b_flute", "c_flute"];

/** Typical thickness per board (CLAUDE.md §6.4, `Board::typical_thickness`): outside it the engine warns. */
export const TYPICAL_THICKNESS_MM: Readonly<Record<Board, { readonly min: number; readonly max: number }>> = {
  paperboard: { min: 0.4, max: 0.6 },
  chipboard: { min: 1.0, max: 2.5 },
  e_flute: { min: 1.2, max: 2.0 },
  b_flute: { min: 2.5, max: 3.5 },
  c_flute: { min: 3.5, max: 4.5 },
};

/** What the engine accepts (`insert_core::limits`): thickness above 0 up to 10 mm, gaps 0–10 mm. */
export const THICKNESS_RANGE_MM = { min: 0.1, max: 10 } as const;
export const CLEARANCE_RANGE_MM = { min: 0, max: 10 } as const;

/** True for corrugated board. */
export function isCorrugated(board: Board): boolean {
  return board === "e_flute" || board === "b_flute" || board === "c_flute";
}

/** Board changes the person made to the starting values (their printer's board). */
export interface BoardTuning {
  readonly thicknessMm: Maybe<number>;
  readonly clearanceMm: Maybe<number>;
}

export const NO_TUNING: BoardTuning = { thicknessMm: none, clearanceMm: none };

/** The board's starting values (CLAUDE.md §6.4) with the person's changes. */
export function materialFor({ board, tuning }: { board: Board; tuning: BoardTuning }): Material {
  const start = STARTING_MATERIALS[board];
  return {
    ...start,
    thickness_mm: tuning.thicknessMm.kind === "some" ? tuning.thicknessMm.value : start.thickness_mm,
    clearance_mm: tuning.clearanceMm.kind === "some" ? tuning.clearanceMm.value : start.clearance_mm,
  };
}

/** Narrowest web of board CLAUDE.md §6.5 accepts: 3 mm, or 3× the thickness when more. */
export function minWebMm(material: Material): number {
  const MIN_WEB_MM = 3;
  const WEB_THICKNESSES = 3;
  return Math.max(MIN_WEB_MM, WEB_THICKNESSES * material.thickness_mm);
}

// ---------------------------------------------------------------- defaults per style

/** A card's or deck's margin: at least the board's web (M1-A7), never under this. */
const BASE_MARGIN_MM = 10;
/** Grid dividers stand this share of the product's height. */
const DIVIDER_SHARE = 0.7;
/** A cradle's deck stands this share of the product's height. */
const DECK_SHARE = 0.4;
/** Lengths the defaults round to: a tenth of a millimetre. */
const TENTHS = 10;

function roundTenth(value: number): number {
  return Math.round(value * TENTHS) / TENTHS;
}

function ceilTenth(value: number): number {
  return Math.ceil(value * TENTHS - 1e-9) / TENTHS;
}

/**
 * The margin round a card's or deck's holes: 10 mm, or the board's web when wider (12 mm for
 * C-flute), so the hole-to-edge warning doesn't fire on a fresh design (M1-A7).
 */
export function marginMm(material: Material): number {
  return ceilTenth(Math.max(BASE_MARGIN_MM, minWebMm(material)));
}

/** Kept above the legs' least height, so rounding never puts the deck just under it. */
const DECK_SLACK_MM = 0.5;

/**
 * A cradle deck's height: 40 % of the product, and never below what the legs need in this
 * board (a riser of at least a web: 1.5 × thickness + web, `LegRules::new`) plus a little.
 */
export function deckHeightMm({ heightMm, material }: { heightMm: number; material: Material }): number {
  const LEG_THICKNESSES = 1.5;
  const legsNeed = LEG_THICKNESSES * material.thickness_mm + minWebMm(material) + DECK_SLACK_MM;
  return Math.max(roundTenth(heightMm * DECK_SHARE), ceilTenth(legsNeed));
}

/** Grid dividers: 70 % of the product's height. */
export function dividerHeightMm(heightMm: number): number {
  return roundTenth(heightMm * DIVIDER_SHARE);
}

// ---------------------------------------------------------------- the design

/** The product's id in the design (one product type in the MVP, CLAUDE.md §10). */
export const PRODUCT_ID = "p1";

/** Everything the person chose. */
export interface DesignChoices {
  readonly heightMm: number;
  readonly quantity: number;
  readonly style: InsertStyle;
  readonly grid: GridLayout;
  readonly material: Material;
  readonly box: BoxChoice;
  readonly boxBoard: Board;
}

/** The insert settings a style starts with for these choices. */
function insertSpec(choices: DesignChoices): InsertSpec {
  switch (choices.style) {
    case InsertStyle.Grid: {
      const single = choices.grid.rows === 1 && choices.grid.columns === 1;
      return {
        style: choices.style,
        rows: choices.grid.rows,
        columns: choices.grid.columns,
        divider_height_mm: dividerHeightMm(choices.heightMm),
        // The box's walls frame the grid; without a box (or with one cell) the grid needs its own.
        outer_frame: choices.box === BoxChoice.None || single,
        finger_notches: true,
      };
    }
    case InsertStyle.Card:
      return {
        style: choices.style,
        margin_mm: marginMm(choices.material),
        finger_notches: { side: "bottom", along: 0.5 },
        align: "center",
        placements: [],
      };
    case InsertStyle.Cradle:
      return {
        style: choices.style,
        deck_height_mm: deckHeightMm({ heightMm: choices.heightMm, material: choices.material }),
        margin_mm: marginMm(choices.material),
        align: "center",
        placements: [],
      };
  }
}

/** The engine's design for the product and the choices: sent to the preview and saved as is. */
export function buildDesign({
  product,
  choices,
}: {
  product: ChosenProduct;
  choices: DesignChoices;
}): Design {
  return {
    material: choices.material,
    products: [
      {
        id: PRODUCT_ID,
        contour_mm: product.contourMm,
        height_mm: choices.heightMm,
        quantity: clampQuantity(choices.quantity),
      },
    ],
    insert: insertSpec(choices),
    outer_box:
      choices.box === BoxChoice.None
        ? null
        : { style: choices.box, material: STARTING_MATERIALS[choices.boxBoard], inside_mm: null },
  };
}

// ---------------------------------------------------------------- what the preview tells

/** The area a closed outline encloses (shoelace), in mm². */
export function polygonAreaMm2(polygon: Polygon): number {
  const twice = polygon.reduce((sum, point, index) => {
    const next = polygon[(index + 1) % polygon.length];
    return sum + point[0] * next[1] - next[0] * point[1];
  }, 0);
  return Math.abs(twice) / 2;
}

/**
 * The share of the box's inside the products leave empty: 1 − product volume ÷ inside
 * volume (CLAUDE.md §6.3, an estimate: the EU method is due in 2028). Products are prisms of
 * their outline. `none` without a box.
 */
export function emptySpaceShare({
  design,
  packaging,
}: {
  design: Design;
  packaging: Packaging;
}): Maybe<number> {
  const box = packaging.outer_box;
  if (box === null) {
    return none;
  }
  const inside = box.inside_mm.length_mm * box.inside_mm.width_mm * box.inside_mm.height_mm;
  if (inside <= 0) {
    return none;
  }
  const products = design.products.reduce(
    (sum, product) => sum + polygonAreaMm2(product.contour_mm) * product.height_mm * product.quantity,
    0,
  );
  return some(Math.min(1, Math.max(0, 1 - products / inside)));
}
