import { existsSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { none, some } from "@/lib/maybe";
import {
  BoxChoice,
  buildDesign,
  clampQuantity,
  deckHeightMm,
  defaultGridLayout,
  emptySpaceShare,
  gridFor,
  gridLayouts,
  marginMm,
  materialFor,
  measuredProduct,
  NO_TUNING,
  parseHeight,
  polygonAreaMm2,
  ProductSource,
  recommendBoard,
  recommendBoxBoard,
  recommendStyle,
  StyleReason,
  typedProduct,
  type ChosenProduct,
  type DesignChoices,
} from "./choices";
import {
  fieldText,
  initialChoices,
  readTuning,
  reduceChoices,
  resolveChoices,
} from "./design-choices-state";
import { boundsOf, fitSummary, layOutPieces, placedOutline, planView } from "./preview";
import { InsertStyle, STARTING_MATERIALS } from "./samples";
import type { Board, Design, Packaging, ProductFit, Warning } from "./types";

const JAR = typedProduct({ shape: "round", diameterMm: 52, heightMm: 95 });
const TIN = typedProduct({ shape: "rectangular", lengthMm: 80, widthMm: 40, heightMm: 18 });

/** Choices for the jar, overridable. */
function choices(overrides: Partial<DesignChoices> = {}): DesignChoices {
  return {
    heightMm: 95,
    quantity: 6,
    style: InsertStyle.Grid,
    grid: { rows: 2, columns: 3 },
    material: STARTING_MATERIALS.e_flute,
    box: BoxChoice.Mailer,
    boxBoard: "e_flute",
    ...overrides,
  };
}

describe("the product", () => {
  it("takes a measured outline as vision gave it, longer side first", () => {
    const product = measuredProduct({
      shape: "rectangular",
      bbox_mm: { l: 30.2, w: 50.1 },
      diameter_mm: null,
      contour_mm: [
        [0, 0],
        [50.1, 0],
        [50.1, 30.2],
        [0, 30.2],
      ],
      contour_source: "detected",
      height_mm: null,
    });
    expect(product.source).toBe(ProductSource.Measured);
    expect([product.lengthMm, product.widthMm]).toEqual([50.1, 30.2]);
    expect(product.round).toBe(false);
    expect(product.heightMm).toEqual(none);
  });

  it("keeps a measured height and an outline drawn by hand", () => {
    const product = measuredProduct({
      shape: "round",
      bbox_mm: { l: 60.3, w: 60.1 },
      diameter_mm: 60.3,
      contour_mm: JAR.contourMm,
      contour_source: "user",
      height_mm: 95,
    });
    expect(product.source).toBe(ProductSource.Drawn);
    expect(product.round).toBe(true);
    expect(product.lengthMm).toBe(60.3);
    expect(product.heightMm).toEqual(some(95));
  });

  it("makes typed sizes into a circle or a rectangle from the origin", () => {
    const box = boundsOf(JAR.contourMm);
    expect(box.x).toBeCloseTo(0);
    expect(box.width).toBeCloseTo(52);
    expect(TIN.contourMm).toEqual([
      [0, 0],
      [80, 0],
      [80, 40],
      [0, 40],
    ]);
    expect(TIN.heightMm).toEqual(some(18));
  });

  it("reads heights vision accepts (0.1–1000 mm), with a comma too", () => {
    expect(parseHeight("95")).toEqual(some(95));
    expect(parseHeight(" 95,5 ")).toEqual(some(95.5));
    expect(parseHeight("0.1")).toEqual(some(0.1));
    expect(parseHeight("0")).toEqual(none);
    expect(parseHeight("1000.1")).toEqual(none);
    expect(parseHeight("")).toEqual(none);
    expect(parseHeight("tall")).toEqual(none);
    expect(fieldText(95.04)).toBe("95");
    expect(fieldText(1234.56)).toBe("1234.6");
  });

  it("keeps quantities whole and within 1–100", () => {
    expect(clampQuantity(0)).toBe(1);
    expect(clampQuantity(6.4)).toBe(6);
    expect(clampQuantity(500)).toBe(100);
    expect(clampQuantity(Number.NaN)).toBe(1);
  });
});

describe("grid layouts", () => {
  it("offers the grids that hold the count, none much longer than wide", () => {
    expect(gridLayouts(6)).toEqual([
      { rows: 1, columns: 6 },
      { rows: 2, columns: 3 },
    ]);
    expect(gridLayouts(12)).toEqual([
      { rows: 2, columns: 6 },
      { rows: 3, columns: 4 },
    ]);
    expect(gridLayouts(1)).toEqual([{ rows: 1, columns: 1 }]);
    expect(gridLayouts(100)).toHaveLength(4);
  });

  it("starts with the fewest empty cells, then the squarest", () => {
    expect(defaultGridLayout(6)).toEqual({ rows: 2, columns: 3 });
    expect(defaultGridLayout(12)).toEqual({ rows: 3, columns: 4 });
    expect(defaultGridLayout(24)).toEqual({ rows: 4, columns: 6 });
    expect(defaultGridLayout(5)).toEqual({ rows: 1, columns: 5 });
    expect(defaultGridLayout(7)).toEqual({ rows: 2, columns: 4 });
    expect(defaultGridLayout(10)).toEqual({ rows: 2, columns: 5 });
  });

  it("always has room for the count", () => {
    for (const quantity of Array.from({ length: 100 }, (_, index) => index + 1)) {
      const layouts = gridLayouts(quantity);
      expect(layouts.length).toBeGreaterThan(0);
      for (const layout of layouts) {
        expect(layout.rows * layout.columns).toBeGreaterThanOrEqual(quantity);
        expect(layout.rows).toBeLessThanOrEqual(layout.columns);
      }
    }
  });

  it("keeps a chosen layout while it's offered, else falls back", () => {
    expect(gridFor({ chosen: some({ rows: 1, columns: 6 }), quantity: 6 })).toEqual({ rows: 1, columns: 6 });
    expect(gridFor({ chosen: some({ rows: 1, columns: 6 }), quantity: 12 })).toEqual({ rows: 3, columns: 4 });
    expect(gridFor({ chosen: none, quantity: 4 })).toEqual({ rows: 2, columns: 2 });
  });
});

describe("recommendations", () => {
  it("suggests a card for low items, a grid for many, a cradle for a few tall ones", () => {
    expect(recommendStyle({ heightMm: 18, quantity: 12 })).toEqual({ style: InsertStyle.Card, reason: StyleReason.Low });
    expect(recommendStyle({ heightMm: 95, quantity: 6 })).toEqual({ style: InsertStyle.Grid, reason: StyleReason.Many });
    expect(recommendStyle({ heightMm: 120, quantity: 2 })).toEqual({ style: InsertStyle.Cradle, reason: StyleReason.Tall });
  });

  it("suggests chipboard for a card, E-flute where board folds or slots, and the box's usual board", () => {
    expect(recommendBoard(InsertStyle.Card)).toBe("chipboard");
    expect(recommendBoard(InsertStyle.Grid)).toBe("e_flute");
    expect(recommendBoard(InsertStyle.Cradle)).toBe("e_flute");
    expect(recommendBoxBoard("mailer")).toBe("e_flute");
    expect(recommendBoxBoard("shipping")).toBe("b_flute");
  });
});

describe("defaults per style", () => {
  it("keeps a margin of at least the board's web (M1-A7)", () => {
    expect(marginMm(STARTING_MATERIALS.e_flute)).toBe(10);
    expect(marginMm(STARTING_MATERIALS.c_flute)).toBe(12);
  });

  it("raises a cradle's deck to what its legs need", () => {
    expect(deckHeightMm({ heightMm: 120, material: STARTING_MATERIALS.e_flute })).toBe(48);
    // 1.5 × 4 mm + a 12 mm web + 0.5 mm: a low product in C-flute.
    expect(deckHeightMm({ heightMm: 20, material: STARTING_MATERIALS.c_flute })).toBe(18.5);
    // 40 % of 18 mm is exactly E-flute's least (7.2 mm): kept clear of it.
    expect(deckHeightMm({ heightMm: 18, material: STARTING_MATERIALS.e_flute })).toBe(7.7);
  });

  it("applies the printer's board over the starting values", () => {
    expect(materialFor({ board: "chipboard", tuning: NO_TUNING })).toEqual(STARTING_MATERIALS.chipboard);
    expect(materialFor({ board: "chipboard", tuning: { thicknessMm: some(2), clearanceMm: none } })).toEqual({
      ...STARTING_MATERIALS.chipboard,
      thickness_mm: 2,
    });
  });
});

describe("the design", () => {
  it("holds one product type with the chosen height and count", () => {
    const design = buildDesign({ product: JAR, choices: choices() });
    expect(design.products).toEqual([{ id: "p1", contour_mm: JAR.contourMm, height_mm: 95, quantity: 6 }]);
    expect(design.material).toEqual(STARTING_MATERIALS.e_flute);
    expect(design.outer_box).toEqual({ style: "mailer", material: STARTING_MATERIALS.e_flute, inside_mm: null });
  });

  it("frames a grid only without a box, or with a single cell", () => {
    const grid = (overrides: Partial<DesignChoices>) => buildDesign({ product: JAR, choices: choices(overrides) }).insert;
    expect(grid({})).toMatchObject({ style: "partition_grid", rows: 2, columns: 3, outer_frame: false, divider_height_mm: 66.5 });
    expect(grid({ box: BoxChoice.None })).toMatchObject({ outer_frame: true });
    expect(grid({ quantity: 1, grid: { rows: 1, columns: 1 } })).toMatchObject({ outer_frame: true });
  });

  it("gives a card notches and a cradle its deck, both with the board's margin", () => {
    const card = buildDesign({ product: TIN, choices: choices({ style: InsertStyle.Card, material: STARTING_MATERIALS.c_flute }) });
    expect(card.insert).toEqual({
      style: "cutout_card",
      margin_mm: 12,
      finger_notches: { side: "bottom", along: 0.5 },
      align: "center",
      placements: [],
    });
    const cradle = buildDesign({ product: JAR, choices: choices({ style: InsertStyle.Cradle, quantity: 2 }) });
    expect(cradle.insert).toMatchObject({ style: "cradle_platform", deck_height_mm: 38, margin_mm: 10 });
  });

  it("leaves the box out for an insert alone", () => {
    expect(buildDesign({ product: JAR, choices: choices({ box: BoxChoice.None }) }).outer_box).toBeNull();
  });

  it("measures area and the box's empty space", () => {
    expect(polygonAreaMm2(TIN.contourMm)).toBe(3200);
    const design = buildDesign({ product: TIN, choices: choices({ quantity: 1, heightMm: 10 }) });
    const packaging = {
      insert: { pieces: [], length_mm: 0, width_mm: 0, height_mm: 0, placements: [], mounts: [] },
      outer_box: {
        style: "mailer",
        pieces: [],
        inside_mm: { length_mm: 100, width_mm: 64, height_mm: 10 },
        outside_mm: { length_mm: 110, width_mm: 70, height_mm: 14 },
        mounts: [],
      },
    } satisfies Packaging;
    expect(emptySpaceShare({ design, packaging })).toEqual(some(0.5));
    expect(emptySpaceShare({ design, packaging: { ...packaging, outer_box: null } })).toEqual(none);
  });
});

describe("the choices page's state", () => {
  it("starts from the product's height, one copy, a mailer", () => {
    const state = initialChoices(JAR);
    expect(state.heightText).toBe("95");
    expect(state.quantity).toBe(1);
    expect(state.box).toBe(BoxChoice.Mailer);
    expect(initialChoices({ ...JAR, heightMm: none }).heightText).toBe("");
  });

  it("follows the recommendations until the person picks", () => {
    let state = initialChoices(JAR);
    expect(resolveChoices(state).style).toBe(InsertStyle.Cradle);
    state = reduceChoices(state, { type: "quantity", quantity: 6 });
    expect(resolveChoices(state)).toMatchObject({ style: InsertStyle.Grid, board: "e_flute", grid: { rows: 2, columns: 3 } });
    state = reduceChoices(state, { type: "style", style: InsertStyle.Card });
    state = reduceChoices(state, { type: "quantity", quantity: 1 });
    expect(resolveChoices(state)).toMatchObject({ style: InsertStyle.Card, board: "chipboard", recommendedBoard: "chipboard" });
  });

  it("waits for a height before there's a design", () => {
    const state = reduceChoices(initialChoices(JAR), { type: "height", text: "" });
    const resolved = resolveChoices(state);
    expect(resolved.choices).toEqual(none);
    expect(resolved.style).toBe(InsertStyle.Card);
  });

  it("clears the board tuning with a new board, and follows a new box's board", () => {
    let state = reduceChoices(initialChoices(JAR), { type: "thickness", text: "1.8" });
    expect(resolveChoices(state).material.thickness_mm).toBe(1.8);
    state = reduceChoices(state, { type: "board", board: "b_flute" });
    expect(state.thicknessText).toBe("");
    expect(resolveChoices(state).material).toEqual(STARTING_MATERIALS.b_flute);
    state = reduceChoices(state, { type: "boxBoard", board: "c_flute" });
    expect(resolveChoices(state).boxBoard).toBe("c_flute");
    state = reduceChoices(state, { type: "box", box: BoxChoice.Shipping });
    expect(resolveChoices(state).boxBoard).toBe("b_flute");
  });

  it("reads tuning fields: empty keeps the start, out of range is refused", () => {
    const range = { min: 0.1, max: 10 };
    expect(readTuning({ text: "", range })).toEqual({ kind: "start" });
    expect(readTuning({ text: "1,4", range })).toEqual({ kind: "set", mm: 1.4 });
    expect(readTuning({ text: "0", range })).toEqual({ kind: "invalid" });
    expect(readTuning({ text: "12", range })).toEqual({ kind: "invalid" });
  });
});

describe("the preview's geometry", () => {
  it("places an outline by its box's centre, turned clockwise on screen", () => {
    const placed = placedOutline({
      contour: TIN.contourMm,
      placement: { product_id: "p1", center_mm: [100, 50], rotation_deg: 90 },
    });
    const box = boundsOf(placed);
    expect(box.x).toBeCloseTo(80);
    expect(box.y).toBeCloseTo(10);
    expect(box.width).toBeCloseTo(40);
    expect(box.height).toBeCloseTo(80);
    // The top-left corner turns to the top-right.
    expect(placed[0][0]).toBeCloseTo(120);
    expect(placed[0][1]).toBeCloseTo(10);
  });

  it("counts fits by colour", () => {
    const fit = (color: ProductFit["color"], clearance: number): ProductFit => ({
      placement: 0,
      color,
      clearance_mm: clearance,
      target_mm: 1.5,
      clashes: [],
    });
    expect(fitSummary([fit("green", 1.5), fit("yellow", 0.4), fit("green", 2)])).toEqual({
      green: 2,
      yellow: 1,
      red: 0,
      tightestMm: 0.4,
    });
  });
});

/**
 * The real engine (M1-A10's WASM, built by `npm run build:wasm`; CI builds it first). Every
 * fresh design the page can start must generate, fit, and pass the manufacturability checks.
 */
const WASM_DIR = fileURLToPath(new URL("../../wasm/insert-core/", import.meta.url));
const WASM_FILE = `${WASM_DIR}insert_core_bg.wasm`;

describe.skipIf(!existsSync(WASM_FILE))("fresh designs in the engine", async () => {
  const glue = await import("@/wasm/insert-core/insert_core");
  glue.initSync({ module: readFileSync(WASM_FILE) });
  const generate = (design: Design) => JSON.parse(glue.generate(JSON.stringify(design))) as Packaging;
  const warnings = (design: Design) => JSON.parse(glue.warnings(JSON.stringify(design), "{}")) as Warning[];
  const fits = (design: Design) => JSON.parse(glue.fits(JSON.stringify(design))) as ProductFit[];

  const products: ReadonlyArray<readonly [string, ChosenProduct]> = [
    ["a 52 mm jar, 95 mm tall", JAR],
    ["an 80 × 40 mm tin, 18 mm tall", TIN],
    ["a 30 mm lipstick, 85 mm tall", typedProduct({ shape: "round", diameterMm: 30, heightMm: 85 })],
  ];
  const boards: readonly Board[] = ["paperboard", "chipboard", "e_flute", "b_flute", "c_flute"];
  /**
   * The warnings a fresh design may show, because they're about the choice rather than the
   * defaults, and the preview counts them: paperboard's slots are under 1 mm (it isn't
   * suggested for slotted styles); products packed at the least web can leave a card or deck
   * more hole than board (the editor spreads them, E5); a product no taller than a cradle's
   * deck sinks below it. A hole near an edge, or anything else, is a bad default.
   */
  const expected = ({ product, style, board }: { product: ChosenProduct; style: InsertStyle; board: Board }) => [
    ...(board === "paperboard" && style !== InsertStyle.Card ? ["too_small"] : []),
    ...(style !== InsertStyle.Grid ? ["weak_panel"] : []),
    ...(style === InsertStyle.Cradle && product.heightMm.kind === "some" && product.heightMm.value <= 40 ? ["product_below_deck"] : []),
  ];

  for (const [name, product] of products) {
    for (const quantity of [1, 2, 6, 12]) {
      for (const style of [InsertStyle.Grid, InsertStyle.Card, InsertStyle.Cradle]) {
        it(`${name} × ${quantity} in a ${style}: generates, fits, nothing to fix`, () => {
          for (const board of boards) {
            for (const box of [BoxChoice.Mailer, BoxChoice.Shipping, BoxChoice.None]) {
              const resolved = resolveChoices({
                ...reduceChoices(reduceChoices(initialChoices(product), { type: "quantity", quantity }), { type: "style", style }),
                board: some(board),
                box,
              });
              expect(resolved.choices.kind).toBe("some");
              if (resolved.choices.kind === "none") {
                return;
              }
              const design = buildDesign({ product, choices: resolved.choices.value });
              const label = `${board}, ${box}`;
              const packaging = generate(design);
              expect(packaging.insert.placements, label).toHaveLength(quantity);
              expect(fits(design).every((fit) => fit.color === "green"), label).toBe(true);
              const found = warnings(design).map((warning) => warning.rule.kind);
              expect(found.filter((kind) => !expected({ product, style, board }).includes(kind)), label).toEqual([]);
              expect(planView({ design, packaging, fits: fits(design) }).products).toHaveLength(quantity);
              expect(layOutPieces({ packaging, gapMm: 12 }).pieces.length).toBeGreaterThan(0);
            }
          }
        });
      }
    }
  }
});
