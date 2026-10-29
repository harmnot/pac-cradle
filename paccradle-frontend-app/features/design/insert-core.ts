import { none, some, type Maybe } from "@/lib/maybe";
import type { CheckOptions, Design, Packaging, ProductFit, Warning } from "./types";

/**
 * The geometry engine in the browser (M1-A10): `insert_core` built to WebAssembly by
 * `npm run build:wasm` into `wasm/insert-core/` (generated, not committed). Editor
 * routes load it on first use; marketing pages never download it.
 *
 * The engine checks and regenerates the design on every call and keeps nothing between
 * calls. `generate` and `fits` are fast enough for every slider frame on normal designs;
 * run `warnings` once an edit settles. Drawn lines come back as plain straight segments,
 * within 0.05 mm of the true lines with points to 0.01 mm (CLAUDE.md §7.4): production
 * files are made only on the server.
 */

/** A call's result: the value, or the engine's message saying what to fix. */
export type EngineResult<T> =
  | { readonly kind: "ok"; readonly value: T }
  | { readonly kind: "error"; readonly message: string };

/** The engine's calls, once loaded. */
export interface InsertCore {
  /** The packaging to draw and fold, rounded for the preview. */
  generate(design: Design): EngineResult<Packaging>;
  /** Every product copy's fit, in placement order. */
  fits(design: Design): EngineResult<readonly ProductFit[]>;
  /** The manufacturability warnings, optionally against a sheet or cutting mat. */
  warnings(request: WarningsRequest): EngineResult<readonly Warning[]>;
}

/** What `InsertCore.warnings` checks. */
export interface WarningsRequest {
  readonly design: Design;
  readonly options: CheckOptions;
}

/** Whether the engine loaded. */
export type LoadResult =
  | { readonly kind: "ok"; readonly core: InsertCore }
  | { readonly kind: "error"; readonly message: string };

/** No sheet: skip the sheet-size rule. */
export const NO_SHEET: CheckOptions = { sheet: null };

/** The engine's JS glue, as wasm-bindgen writes it. */
type Glue = typeof import("@/wasm/insert-core/insert_core");

let loaded: Maybe<Promise<LoadResult>> = none;

/**
 * Loads the engine once and shares it. A failed load (offline, blocked download) is
 * forgotten so the next call tries again.
 */
export async function loadInsertCore(): Promise<LoadResult> {
  if (loaded.kind === "some") {
    return loaded.value;
  }
  const loading = load();
  loaded = some(loading);
  const result = await loading;
  if (result.kind === "error") {
    loaded = none;
  }
  return result;
}

/** Downloads and starts the WebAssembly module, then wraps its calls. */
async function load(): Promise<LoadResult> {
  try {
    const glue = await import("@/wasm/insert-core/insert_core");
    // With no argument, the glue fetches insert_core_bg.wasm from next to itself.
    await glue.default();
    return { kind: "ok", core: wrap(glue) };
  } catch (error) {
    return { kind: "error", message: messageOf(error) };
  }
}

/** Typed calls over the glue's JSON-string functions. */
function wrap(glue: Glue): InsertCore {
  return {
    generate: (design) => call<Packaging>(() => glue.generate(JSON.stringify(design))),
    fits: (design) => call<readonly ProductFit[]>(() => glue.fits(JSON.stringify(design))),
    warnings: ({ design, options }) =>
      call<readonly Warning[]>(() =>
        glue.warnings(JSON.stringify(design), JSON.stringify(options)),
      ),
  };
}

/** Runs one engine call; the engine throws an `Error` when a design can't be generated. */
function call<T>(run: () => string): EngineResult<T> {
  try {
    return { kind: "ok", value: JSON.parse(run()) as T };
  } catch (error) {
    return { kind: "error", message: messageOf(error) };
  }
}

function messageOf(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
