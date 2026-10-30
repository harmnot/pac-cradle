import { useCallback, useEffect, useReducer } from "react";
import { ClientErrorCode, type ApiError } from "@/lib/api-client";
import { none, some, type Maybe } from "@/lib/maybe";
import { getCapture } from "./api";
import { CaptureStatus, type Capture } from "./types";

/**
 * A capture, polled about once a second while vision measures it (MILESTONES M1-E2), then
 * left alone. The signed photo link it carries lasts 10 minutes: call `reload()` for a new one.
 * Each answer signs a new link, so the one the page shows is kept while it still works: the
 * photo isn't downloaded again on every poll.
 */

/** Poll this often while `pending`. */
const POLL_MS = 1000;
/** After this many polls still pending, the page says it's taking longer than usual. */
const SLOW_AFTER_POLLS = 15;
/** A photo link is kept while it works for at least this much longer. */
const PHOTO_LINK_MARGIN_MS = 60_000;

/** What the page shows. */
export interface CaptureQuery {
  readonly capture: Maybe<Capture>;
  /** The last call failed (the capture as last read stays). */
  readonly error: Maybe<ApiError>;
  /** Pending for longer than a run usually takes (the worker may be down). */
  readonly slow: boolean;
  readonly reload: () => void;
  /** Shows a capture a correction answered with, polling again when a run is pending. */
  readonly apply: (capture: Capture) => void;
}

interface State {
  readonly capture: Maybe<Capture>;
  readonly error: Maybe<ApiError>;
  readonly polls: number;
}

type Action =
  /** A capture was read at `now` (ms since the epoch). */
  | { readonly type: "read"; readonly capture: Capture; readonly now: number }
  | { readonly type: "failed"; readonly error: ApiError }
  | { readonly type: "restart" };

/** The new capture, with the photo link already shown while that one still works. */
export function keepPhotoLink({ shown, next, now }: { shown: Maybe<Capture>; next: Capture; now: number }): Capture {
  if (shown.kind === "none" || shown.value.photo.kind === "none" || next.photo.kind === "none") {
    return next;
  }
  const kept = shown.value.photo.value;
  return kept.expiresAt - now > PHOTO_LINK_MARGIN_MS ? { ...next, photo: some(kept) } : next;
}

function reduce(state: State, action: Action): State {
  switch (action.type) {
    case "read":
      return {
        capture: some(keepPhotoLink({ shown: state.capture, next: action.capture, now: action.now })),
        error: none,
        polls: state.polls + 1,
      };
    case "failed":
      return { ...state, error: some(action.error) };
    case "restart":
      return { ...state, error: none, polls: 0 };
  }
}

/** Reads capture `id`, polling while it is pending. */
export function useCapture(id: string): CaptureQuery {
  const [state, dispatch] = useReducer(reduce, { capture: none, error: none, polls: 0 });
  const [round, restart] = useReducer((count: number) => count + 1, 0);

  useEffect(() => {
    const controller = new AbortController();
    let timer: ReturnType<typeof setTimeout> | undefined;
    const poll = async () => {
      const result = await getCapture({ id, signal: controller.signal });
      if (result.kind === "error") {
        if (result.error.code !== ClientErrorCode.Aborted) {
          dispatch({ type: "failed", error: result.error });
        }
        return;
      }
      dispatch({ type: "read", capture: result.value, now: Date.now() });
      if (result.value.status === CaptureStatus.Pending) {
        timer = setTimeout(() => void poll(), POLL_MS);
      }
    };
    void poll();
    return () => {
      controller.abort();
      clearTimeout(timer);
    };
  }, [id, round]);

  const reload = useCallback(() => {
    dispatch({ type: "restart" });
    restart();
  }, []);

  const apply = useCallback((capture: Capture) => {
    dispatch({ type: "restart" });
    dispatch({ type: "read", capture, now: Date.now() });
    if (capture.status === CaptureStatus.Pending) {
      restart();
    }
  }, []);

  const pending =
    state.capture.kind === "some" && state.capture.value.status === CaptureStatus.Pending;
  return {
    capture: state.capture,
    error: state.error,
    slow: pending && state.polls > SLOW_AFTER_POLLS,
    reload,
    apply,
  };
}
