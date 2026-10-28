/**
 * A value that may be missing, used instead of null/undefined (code-style rule).
 */
export type Maybe<T> = { readonly kind: "some"; readonly value: T } | { readonly kind: "none" };

/** Wraps a present value. */
export function some<T>(value: T): Maybe<T> {
  return { kind: "some", value };
}

/** The missing value. */
export const none: Maybe<never> = { kind: "none" };

/** Converts an optional environment value; empty or whitespace-only strings count as missing. */
export function fromEnv(value: string | undefined): Maybe<string> {
  const trimmed = (value ?? "").trim();
  return trimmed === "" ? none : some(trimmed);
}

/** Returns the value, or the fallback when it is missing. */
export function valueOr<T>(maybe: Maybe<T>, fallback: T): T {
  return maybe.kind === "some" ? maybe.value : fallback;
}
