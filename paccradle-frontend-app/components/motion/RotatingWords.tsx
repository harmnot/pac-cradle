"use client";

import { useEffect, useState, type CSSProperties } from "react";
import styles from "./RotatingWords.module.css";

const DEFAULT_INTERVAL_MS = 2600;
const REDUCED_MOTION_QUERY = "(prefers-reduced-motion: reduce)";

/** Props for a cycling word. */
export interface RotatingWordsProps {
  readonly words: readonly string[];
  readonly intervalMs?: number;
  /** Position in the surrounding word stagger, for the entry animation. */
  readonly wordIndex?: number;
}

type WordState = "current" | "previous" | "next";

interface StateQuery {
  readonly position: number;
  readonly current: number;
  readonly count: number;
}

function stateOf({ position, current, count }: StateQuery): WordState {
  if (position === current) {
    return "current";
  }
  return position === (current - 1 + count) % count ? "previous" : "next";
}

/**
 * Cycles through words with a vertical slide; the box keeps the width of the longest word,
 * so nothing around it jumps. Decorative: give screen readers the full sentence elsewhere.
 * Stays on the first word with reduced motion and while the tab is hidden.
 */
export function RotatingWords({
  words,
  intervalMs = DEFAULT_INTERVAL_MS,
  wordIndex = 0,
}: RotatingWordsProps) {
  const [current, setCurrent] = useState(0);

  useEffect(() => {
    if (words.length < 2 || window.matchMedia(REDUCED_MOTION_QUERY).matches) {
      return;
    }
    const timer = window.setInterval(() => {
      if (document.visibilityState === "visible") {
        setCurrent((value) => (value + 1) % words.length);
      }
    }, intervalMs);
    return () => window.clearInterval(timer);
  }, [words.length, intervalMs]);

  return (
    <span
      aria-hidden="true"
      className={styles.rotator}
      style={{ "--word-index": wordIndex } as CSSProperties}
    >
      {words.map((word, position) => (
        <span
          key={word}
          className={styles.word}
          data-state={stateOf({ position, current, count: words.length })}
        >
          {word}
        </span>
      ))}
    </span>
  );
}
