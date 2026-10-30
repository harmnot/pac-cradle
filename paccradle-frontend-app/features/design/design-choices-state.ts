import { none, some, type Maybe } from "@/lib/maybe";
import {
  BoxChoice,
  CLEARANCE_RANGE_MM,
  clampQuantity,
  gridFor,
  materialFor,
  parseHeight,
  recommendBoard,
  recommendBoxBoard,
  recommendStyle,
  THICKNESS_RANGE_MM,
  type ChosenProduct,
  type DesignChoices,
  type GridLayout,
  type StyleReason,
} from "./choices";
import { InsertStyle } from "./samples";
import type { Board, Material } from "./types";

/**
 * The design choices page's state (M1-E4): what the person typed and picked, as a reducer.
 * Anything not picked yet follows the recommendation (a style for the height and count, a
 * board for the style, a board for the box), so the page always shows a sensible design.
 */

/** What the person has typed and picked. `none` means "follow the recommendation". */
export interface ChoicesState {
  readonly heightText: string;
  readonly quantity: number;
  readonly style: Maybe<InsertStyle>;
  readonly grid: Maybe<GridLayout>;
  readonly board: Maybe<Board>;
  /** The printer's board, when it differs from the starting values ("" keeps them). */
  readonly thicknessText: string;
  readonly clearanceText: string;
  readonly box: BoxChoice;
  readonly boxBoard: Maybe<Board>;
  /** The design's name once edited; until then the page suggests one. */
  readonly name: Maybe<string>;
}

export type ChoicesAction =
  | { readonly type: "height"; readonly text: string }
  | { readonly type: "quantity"; readonly quantity: number }
  | { readonly type: "style"; readonly style: InsertStyle }
  | { readonly type: "grid"; readonly grid: GridLayout }
  | { readonly type: "board"; readonly board: Board }
  | { readonly type: "thickness"; readonly text: string }
  | { readonly type: "clearance"; readonly text: string }
  | { readonly type: "box"; readonly box: BoxChoice }
  | { readonly type: "boxBoard"; readonly board: Board }
  | { readonly type: "name"; readonly name: string };

/** A length as a number field holds it: a tenth of a millimetre at most, no grouping. */
export function fieldText(mm: number): string {
  const TENTHS = 10;
  return String(Math.round(mm * TENTHS) / TENTHS);
}

/** The page's first state: the product's known height, one copy, a mailer box. */
export function initialChoices(product: ChosenProduct): ChoicesState {
  return {
    heightText: product.heightMm.kind === "some" ? fieldText(product.heightMm.value) : "",
    quantity: 1,
    style: none,
    grid: none,
    board: none,
    thicknessText: "",
    clearanceText: "",
    box: BoxChoice.Mailer,
    boxBoard: none,
    name: none,
  };
}

/** Applies one change. A new board clears the board tuning; a new box follows its usual board. */
export function reduceChoices(state: ChoicesState, action: ChoicesAction): ChoicesState {
  switch (action.type) {
    case "height":
      return { ...state, heightText: action.text };
    case "quantity":
      return { ...state, quantity: clampQuantity(action.quantity) };
    case "style":
      return { ...state, style: some(action.style) };
    case "grid":
      return { ...state, grid: some(action.grid) };
    case "board":
      return { ...state, board: some(action.board), thicknessText: "", clearanceText: "" };
    case "thickness":
      return { ...state, thicknessText: action.text };
    case "clearance":
      return { ...state, clearanceText: action.text };
    case "box":
      return { ...state, box: action.box, boxBoard: none };
    case "boxBoard":
      return { ...state, boxBoard: some(action.board) };
    case "name":
      return { ...state, name: some(action.name) };
  }
}

/** A tuning field: empty keeps the starting value; otherwise a number within `range`. */
export type TuningField =
  | { readonly kind: "start" }
  | { readonly kind: "set"; readonly mm: number }
  | { readonly kind: "invalid" };

/** Reads a tuning field's text. */
export function readTuning({ text, range }: { text: string; range: { readonly min: number; readonly max: number } }): TuningField {
  const trimmed = text.trim();
  if (trimmed === "") {
    return { kind: "start" };
  }
  const value = Number.parseFloat(trimmed.replace(",", "."));
  return Number.isFinite(value) && value >= range.min && value <= range.max ? { kind: "set", mm: value } : { kind: "invalid" };
}

function tuned(field: TuningField): Maybe<number> {
  return field.kind === "set" ? some(field.mm) : none;
}

/** What the state comes to. */
export interface ResolvedChoices {
  readonly heightMm: Maybe<number>;
  /** The suggested style and why, once the height is known. */
  readonly recommendation: Maybe<{ readonly style: InsertStyle; readonly reason: StyleReason }>;
  readonly style: InsertStyle;
  readonly grid: GridLayout;
  readonly board: Board;
  readonly recommendedBoard: Board;
  readonly thickness: TuningField;
  readonly clearance: TuningField;
  readonly material: Material;
  readonly boxBoard: Board;
  /** Everything the design needs, once the height is known. */
  readonly choices: Maybe<DesignChoices>;
}

/** Fills in what wasn't picked with the recommendations, and gathers the design's choices. */
export function resolveChoices(state: ChoicesState): ResolvedChoices {
  const heightMm = parseHeight(state.heightText);
  const recommendation =
    heightMm.kind === "some" ? some(recommendStyle({ heightMm: heightMm.value, quantity: state.quantity })) : none;
  const style =
    state.style.kind === "some"
      ? state.style.value
      : recommendation.kind === "some"
        ? recommendation.value.style
        : InsertStyle.Card;
  const recommendedBoard = recommendBoard(style);
  const board = state.board.kind === "some" ? state.board.value : recommendedBoard;
  const thickness = readTuning({ text: state.thicknessText, range: THICKNESS_RANGE_MM });
  const clearance = readTuning({ text: state.clearanceText, range: CLEARANCE_RANGE_MM });
  const material = materialFor({ board, tuning: { thicknessMm: tuned(thickness), clearanceMm: tuned(clearance) } });
  const grid = gridFor({ chosen: state.grid, quantity: state.quantity });
  const boxBoard =
    state.boxBoard.kind === "some"
      ? state.boxBoard.value
      : recommendBoxBoard(state.box === BoxChoice.Shipping ? "shipping" : "mailer");
  return {
    heightMm,
    recommendation,
    style,
    grid,
    board,
    recommendedBoard,
    thickness,
    clearance,
    material,
    boxBoard,
    choices:
      heightMm.kind === "some"
        ? some({ heightMm: heightMm.value, quantity: state.quantity, style, grid, material, box: state.box, boxBoard })
        : none,
  };
}
