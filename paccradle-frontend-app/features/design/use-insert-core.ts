import { useCallback, useEffect, useReducer } from "react";
import { loadInsertCore, type InsertCore } from "./insert-core";

/**
 * The geometry engine for a component: loads the WebAssembly on first use (editor routes
 * only, CLAUDE.md §7.1) and offers a retry when the download fails.
 */

/** Where the engine is. */
export type EngineState =
  | { readonly kind: "loading" }
  | { readonly kind: "failed"; readonly message: string }
  | { readonly kind: "ready"; readonly core: InsertCore };

/** The engine and a way to try loading it again. */
export interface EngineQuery {
  readonly engine: EngineState;
  readonly retry: () => void;
}

type Action = { readonly type: "loaded"; readonly state: EngineState } | { readonly type: "retry" };

function reduce(state: EngineState, action: Action): EngineState {
  switch (action.type) {
    case "loaded":
      return action.state;
    case "retry":
      return { kind: "loading" };
  }
}

/** Loads the engine (shared by every caller) and follows it. */
export function useInsertCore(): EngineQuery {
  const [engine, dispatch] = useReducer(reduce, { kind: "loading" });
  const [attempt, again] = useReducer((count: number) => count + 1, 0);

  useEffect(() => {
    let live = true;
    const load = async () => {
      const result = await loadInsertCore();
      if (!live) {
        return;
      }
      dispatch({
        type: "loaded",
        state: result.kind === "ok" ? { kind: "ready", core: result.core } : { kind: "failed", message: result.message },
      });
    };
    void load();
    return () => {
      live = false;
    };
  }, [attempt]);

  const retry = useCallback(() => {
    dispatch({ type: "retry" });
    again();
  }, []);

  return { engine, retry };
}
