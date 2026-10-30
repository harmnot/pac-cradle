import { useEffect, useMemo, useReducer, type Dispatch } from "react";
import { none, some, type Maybe } from "@/lib/maybe";
import {
  crossingEdges,
  liveSize,
  MIN_OUTLINE_AREA_MM2,
  toPaperMm,
  type LiveSize,
  type PaperMap,
} from "./geometry";
import { editOutline, isChanged, startEditing, type EditorAction, type EditorState } from "./outline-editor";
import type { Point } from "./types";

/**
 * The outline editor (MILESTONES M1-E3): its state, what the outline measures as it stands,
 * and what would stop vision taking it. Start it with `dispatch({ type: "begin", points })`.
 * While `active`, ⌘Z / Ctrl+Z undo and ⇧⌘Z / Ctrl+Y redo anywhere on the page.
 */

const NO_POINTS: readonly Point[] = [];

/** Why the outline can't be sent as it is. */
export const OutlineProblem = {
  /** Two edges cross or touch: vision refuses it (`CAPTURE_OUTLINE_CROSSES_ITSELF`). */
  Crosses: "crosses",
  /** Under 25 mm²: vision refuses it (`CAPTURE_INVALID_INPUT`). */
  TooSmall: "too_small",
} as const;

export type OutlineProblem = (typeof OutlineProblem)[keyof typeof OutlineProblem];

/** What the editor gives the figure and the toolbar. */
export interface OutlineEditing {
  readonly state: EditorState;
  readonly dispatch: Dispatch<EditorAction>;
  /** The edges (by their first point) that cross another. */
  readonly crossing: readonly number[];
  /** The size as drawn, when the paper is known. */
  readonly size: Maybe<LiveSize>;
  readonly problem: Maybe<OutlineProblem>;
  readonly changed: boolean;
}

/** What vision would say about the outline, first problem first. */
function problemOf({ crossing, size }: { crossing: readonly number[]; size: Maybe<LiveSize> }): Maybe<OutlineProblem> {
  if (crossing.length > 0) {
    return some(OutlineProblem.Crosses);
  }
  return size.kind === "some" && size.value.areaMm2 < MIN_OUTLINE_AREA_MM2 ? some(OutlineProblem.TooSmall) : none;
}

/** Whether a key press is undo (`-1`), redo (`1`) or neither (`0`). */
function historyKey(event: KeyboardEvent): -1 | 0 | 1 {
  if (!(event.metaKey || event.ctrlKey) || event.altKey) {
    return 0;
  }
  const key = event.key.toLowerCase();
  if (key === "z") {
    return event.shiftKey ? 1 : -1;
  }
  return key === "y" && event.ctrlKey ? 1 : 0;
}

/** An outline editor (photo pixels) over the sheet `map` maps. */
export function useOutlineEditing({ map, active }: { map: Maybe<PaperMap>; active: boolean }): OutlineEditing {
  const [state, dispatch] = useReducer(editOutline, NO_POINTS, startEditing);

  useEffect(() => {
    if (!active) {
      return;
    }
    const onKey = (event: KeyboardEvent) => {
      const step = historyKey(event);
      if (step === 0) {
        return;
      }
      event.preventDefault();
      dispatch({ type: step === -1 ? "undo" : "redo" });
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [active]);

  const crossing = useMemo(() => crossingEdges(state.points), [state.points]);
  const size = useMemo<Maybe<LiveSize>>(() => {
    if (map.kind === "none") {
      return none;
    }
    const mm = state.points.map((point) => toPaperMm({ map: map.value, point }));
    const all = mm.flatMap((point) => (point.kind === "some" ? [point.value] : []));
    return all.length === mm.length ? some(liveSize(all)) : none;
  }, [state.points, map]);

  return { state, dispatch, crossing, size, problem: problemOf({ crossing, size }), changed: isChanged(state) };
}
