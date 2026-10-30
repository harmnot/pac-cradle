import { none, some, type Maybe } from "@/lib/maybe";
import type { Point } from "./types";

/**
 * The outline editor's state (MILESTONES M1-E3, CLAUDE.md §3 step 4): the points being
 * dragged, the selected one, and an undo/redo history (DEC-05: a reducer that keeps it).
 * Points are in photo pixels, the frame vision's `contour_px` and `edited_contour_px` use.
 */

/** Fewest and most points vision takes in an edited outline (`edit::MAX_EDIT_POINTS`). */
export const MIN_OUTLINE_POINTS = 3;
export const MAX_OUTLINE_POINTS = 2000;
/** Undo steps kept. */
const HISTORY_LIMIT = 100;

type Points = readonly Point[];

/** What the editor holds. */
export interface EditorState {
  /** The outline as it was when editing began (Reset goes back to it). */
  readonly original: Points;
  readonly points: Points;
  readonly selected: Maybe<number>;
  /** A drag is under way; `moved` once the point has left its place. */
  readonly drag: Maybe<{ readonly moved: boolean }>;
  /** Earlier outlines, newest last. */
  readonly past: readonly Points[];
  /** Outlines undone, next first. */
  readonly future: readonly Points[];
  /** The point the last keyboard nudge moved: further nudges of it join that undo step. */
  readonly nudging: Maybe<number>;
}

/** What can happen to it. */
export type EditorAction =
  /** Starts editing `points` afresh, history empty. */
  | { readonly type: "begin"; readonly points: Points }
  /** Picks point `index` up to drag it. */
  | { readonly type: "grab"; readonly index: number }
  /** Adds a point at `at` after point `after` and picks it up. */
  | { readonly type: "insert"; readonly after: number; readonly at: Point }
  /** Moves the point being dragged. */
  | { readonly type: "drag"; readonly to: Point }
  | { readonly type: "release" }
  /** Puts the selected point at `to` in one step (arrow keys). */
  | { readonly type: "nudge"; readonly to: Point }
  | { readonly type: "select"; readonly index: Maybe<number> }
  /** Selects the next (+1) or previous (−1) point. */
  | { readonly type: "step"; readonly by: 1 | -1 }
  | { readonly type: "remove" }
  | { readonly type: "undo" }
  | { readonly type: "redo" }
  | { readonly type: "reset" };

/** A fresh editor for `points`. */
export function startEditing(points: Points): EditorState {
  return { original: points, points, selected: none, drag: none, past: [], future: [], nudging: none };
}

/** The state with `points` as a new undo step. */
function remember({ state, points }: { state: EditorState; points: Points }): EditorState {
  return {
    ...state,
    points,
    past: [...state.past, state.points].slice(-HISTORY_LIMIT),
    future: [],
    nudging: none,
  };
}

/** `points` with point `index` at `to`. */
function moved({ points, index, to }: { points: Points; index: number; to: Point }): Points {
  return points.map((point, at) => (at === index ? to : point));
}

/** Applies one action. */
export function editOutline(state: EditorState, action: EditorAction): EditorState {
  switch (action.type) {
    case "begin":
      return startEditing(action.points);
    case "grab": {
      if (action.index < 0 || action.index >= state.points.length) {
        return state;
      }
      // The undo step is taken now and dropped on release if the point never moved.
      return { ...remember({ state, points: state.points }), selected: some(action.index), drag: some({ moved: false }) };
    }
    case "insert": {
      if (state.points.length >= MAX_OUTLINE_POINTS || action.after < 0 || action.after >= state.points.length) {
        return state;
      }
      const index = action.after + 1;
      const points = [...state.points.slice(0, index), action.at, ...state.points.slice(index)];
      return { ...remember({ state, points }), selected: some(index), drag: some({ moved: true }) };
    }
    case "drag": {
      if (state.drag.kind === "none" || state.selected.kind === "none") {
        return state;
      }
      return {
        ...state,
        points: moved({ points: state.points, index: state.selected.value, to: action.to }),
        drag: some({ moved: true }),
      };
    }
    case "release": {
      if (state.drag.kind === "none") {
        return state;
      }
      const unchanged = !state.drag.value.moved;
      return { ...state, drag: none, past: unchanged ? state.past.slice(0, -1) : state.past };
    }
    case "nudge": {
      if (state.selected.kind === "none") {
        return state;
      }
      const index = state.selected.value;
      const points = moved({ points: state.points, index, to: action.to });
      const joining = state.nudging.kind === "some" && state.nudging.value === index;
      return joining ? { ...state, points } : { ...remember({ state, points }), nudging: some(index) };
    }
    case "select":
      return { ...state, selected: action.index, nudging: none };
    case "step": {
      const count = state.points.length;
      const from = state.selected.kind === "some" ? state.selected.value : action.by === 1 ? -1 : 0;
      return { ...state, selected: some((from + action.by + count) % count), nudging: none };
    }
    case "remove": {
      if (state.selected.kind === "none" || state.points.length <= MIN_OUTLINE_POINTS) {
        return state;
      }
      const index = state.selected.value;
      const points = state.points.filter((_, at) => at !== index);
      return { ...remember({ state, points }), selected: some(index % points.length) };
    }
    case "undo": {
      const previous = state.past.at(-1);
      if (previous === undefined) {
        return state;
      }
      return {
        ...state,
        points: previous,
        past: state.past.slice(0, -1),
        future: [state.points, ...state.future],
        selected: keepSelection({ selected: state.selected, count: previous.length }),
        drag: none,
        nudging: none,
      };
    }
    case "redo": {
      const [next, ...rest] = state.future;
      if (next === undefined) {
        return state;
      }
      return {
        ...state,
        points: next,
        past: [...state.past, state.points],
        future: rest,
        selected: keepSelection({ selected: state.selected, count: next.length }),
        drag: none,
        nudging: none,
      };
    }
    case "reset":
      return samePoints({ a: state.points, b: state.original })
        ? state
        : { ...remember({ state, points: state.original }), selected: none };
  }
}

/** The selection, if that point still exists. */
function keepSelection({ selected, count }: { selected: Maybe<number>; count: number }): Maybe<number> {
  return selected.kind === "some" && selected.value < count ? selected : none;
}

/** Whether two outlines are the same points in the same order. */
export function samePoints({ a, b }: { a: Points; b: Points }): boolean {
  return a.length === b.length && a.every(([x, y], index) => x === b[index][0] && y === b[index][1]);
}

/** Whether the outline differs from where editing began. */
export function isChanged(state: EditorState): boolean {
  return !samePoints({ a: state.points, b: state.original });
}

/** Steps per photo pixel sent to vision: hundredths, as vision rounds its own outlines. */
const SENT_STEPS_PER_PX = 100;

/** The points as sent to vision, in photo pixels to a hundredth. */
export function outlineToSend(points: Points): Points {
  const round = (value: number) => Math.round(value * SENT_STEPS_PER_PX) / SENT_STEPS_PER_PX;
  return points.map(([x, y]) => [round(x), round(y)]);
}
