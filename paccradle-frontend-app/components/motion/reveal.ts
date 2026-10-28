import type { CSSProperties } from "react";

/** Delay between items in a staggered scroll reveal. */
export const REVEAL_STAGGER_MS = 110;

/** Inline style that delays an item's reveal by its position in a list (see styles/motion.css). */
export function revealDelay(index: number, stepMs: number = REVEAL_STAGGER_MS): CSSProperties {
  return { "--reveal-delay": `${index * stepMs}ms` } as CSSProperties;
}
