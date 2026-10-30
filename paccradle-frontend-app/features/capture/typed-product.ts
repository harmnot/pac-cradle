import { useMemo, useSyncExternalStore } from "react";
import { none, some, type Maybe } from "@/lib/maybe";

/**
 * Sizes typed in by hand (CLAUDE.md §3 step 1c, no camera): kept for this tab until the
 * design choices (M1-E4) turn them into a product. Sizes only, never personal data.
 */

/** Shapes a typed product can have. */
export const TypedShape = {
  Round: "round",
  Rectangular: "rectangular",
} as const;

export type TypedShape = (typeof TypedShape)[keyof typeof TypedShape];

/** A product typed in, in millimetres. */
export type TypedProduct =
  | { readonly shape: "round"; readonly diameterMm: number; readonly heightMm: number }
  | {
      readonly shape: "rectangular";
      readonly lengthMm: number;
      readonly widthMm: number;
      readonly heightMm: number;
    };

/** Size limits for typed products, as the API checks heights (0.1–1000 mm). */
export const TYPED_MIN_MM = 1;
export const TYPED_MAX_MM = 1000;

const STORAGE_KEY = "paccradle.typedProduct";

/** Whether `value` is a size the app accepts. */
export function validSize(value: number): boolean {
  return Number.isFinite(value) && value >= TYPED_MIN_MM && value <= TYPED_MAX_MM;
}

/** Keeps the typed product for the next step. */
export function keepTypedProduct(product: TypedProduct): void {
  try {
    window.sessionStorage.setItem(STORAGE_KEY, JSON.stringify(product));
  } catch {
    // Storage is off: the next step asks again.
  }
}

/** The kept product's JSON, or "" when there's none (or storage is off). Browser only. */
function keptText(): string {
  try {
    return window.sessionStorage.getItem(STORAGE_KEY) ?? "";
  } catch {
    return "";
  }
}

/** A kept product's JSON, read back. */
function parseKept(text: string): Maybe<TypedProduct> {
  if (text === "") {
    return none;
  }
  try {
    return some(JSON.parse(text) as TypedProduct);
  } catch {
    return none;
  }
}

/** While the server renders and React hydrates, the tab's storage isn't read yet. */
const NOT_READ = "\u0000not-read";

/** Nothing to listen for: the kept product changes only through `keepTypedProduct` on another page. */
function subscribeNever(): () => void {
  return () => undefined;
}

function serverText(): string {
  return NOT_READ;
}

/** The kept product as a component sees it: not read yet (server, hydration), then read. */
export type KeptTypedProduct =
  | { readonly kind: "reading" }
  | { readonly kind: "read"; readonly product: Maybe<TypedProduct> };

/**
 * The typed product kept in this tab, for a component: "reading" on the server and while
 * React hydrates, so the server's HTML and the first client render agree, then what's kept.
 */
export function useKeptTypedProduct(): KeptTypedProduct {
  const text = useSyncExternalStore(subscribeNever, keptText, serverText);
  return useMemo(() => (text === NOT_READ ? { kind: "reading" } : { kind: "read", product: parseKept(text) }), [text]);
}
