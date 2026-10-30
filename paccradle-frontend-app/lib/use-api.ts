import { useCallback, useEffect, useReducer, useRef } from "react";
import { ClientErrorCode, type ApiError, type ApiResult } from "./api-client";
import { none, some, type Maybe } from "./maybe";

/**
 * Reads from the API in a component (frontend-tech-stack §2: native fetch + small hooks with
 * an in-memory cache, retries and abort). A key already read shows its last value at once and
 * refreshes behind it; leaving the page, or changing the key, aborts the call. Retries for
 * network errors happen in `lib/api-client`.
 */

/** What a query shows. */
export interface QueryState<T> {
  /** `loading` only while nothing is known yet; a cached value shows as `ok` while it refreshes. */
  readonly status: "loading" | "ok" | "error";
  readonly value: Maybe<T>;
  readonly error: Maybe<ApiError>;
  /** A call is running (first load or refresh). */
  readonly busy: boolean;
}

/** A query and a way to run it again. */
export type Query<T> = QueryState<T> & { readonly reload: () => void };

/** What to read. */
export interface QueryOptions<T> {
  /** Names the data in the cache, e.g. `"/me"` or `"/designs?page=1"`. */
  readonly key: string;
  readonly load: (signal: AbortSignal) => Promise<ApiResult<T>>;
  /** Wait (no call) while false, e.g. until a session exists. Default true. */
  readonly enabled?: boolean;
}

const cache = new Map<string, unknown>();

/** Forgets every cached answer (another account signed in, or nobody). */
export function clearQueryCache(): void {
  cache.clear();
}

type Action<T> =
  | { readonly type: "start"; readonly cached: Maybe<T> }
  | { readonly type: "ok"; readonly value: T }
  | { readonly type: "error"; readonly error: ApiError };

function reduce<T>(state: QueryState<T>, action: Action<T>): QueryState<T> {
  switch (action.type) {
    case "start":
      return action.cached.kind === "some"
        ? { status: "ok", value: action.cached, error: none, busy: true }
        : { ...state, status: state.value.kind === "some" ? state.status : "loading", busy: true };
    case "ok":
      return { status: "ok", value: some(action.value), error: none, busy: false };
    case "error":
      return { ...state, status: "error", error: some(action.error), busy: false };
  }
}

function cached<T>(key: string): Maybe<T> {
  return cache.has(key) ? some(cache.get(key) as T) : none;
}

/** Reads `key` with `load`, keeps it cached, and aborts when the component goes. */
export function useApiQuery<T>({ key, load, enabled = true }: QueryOptions<T>): Query<T> {
  const [state, dispatch] = useReducer(reduce<T>, {
    status: "loading",
    value: cached<T>(key),
    error: none,
    busy: false,
  });
  const [version, bump] = useReducer((count: number) => count + 1, 0);
  const loadRef = useRef(load);

  useEffect(() => {
    loadRef.current = load;
  });

  useEffect(() => {
    if (!enabled) {
      return;
    }
    const controller = new AbortController();
    const run = async () => {
      dispatch({ type: "start", cached: cached<T>(key) });
      const result = await loadRef.current(controller.signal);
      if (result.kind === "ok") {
        cache.set(key, result.value);
        dispatch({ type: "ok", value: result.value });
        return;
      }
      if (result.error.code !== ClientErrorCode.Aborted) {
        dispatch({ type: "error", error: result.error });
      }
    };
    void run();
    return () => controller.abort();
  }, [key, enabled, version]);

  const reload = useCallback(() => bump(), []);
  return { ...state, reload };
}
