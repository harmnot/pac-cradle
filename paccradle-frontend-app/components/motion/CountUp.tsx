"use client";

import { useEffect, useState } from "react";

const REDUCED_MOTION_QUERY = "(prefers-reduced-motion: reduce)";
const LOCALE = "en-US";
const DEFAULT_DURATION_MS = 1400;

/** Props for a counting number. */
export interface CountUpProps {
  readonly value: number;
  readonly decimals?: number;
  /** Wait this long after mounting before counting. */
  readonly delayMs?: number;
  readonly durationMs?: number;
}

function easeOutCubic(progress: number): number {
  return 1 - (1 - progress) ** 3;
}

/**
 * A number that counts up from zero once, like a measurement read-out settling.
 * Renders the final value on the server and with reduced motion.
 */
export function CountUp({
  value,
  decimals = 0,
  delayMs = 0,
  durationMs = DEFAULT_DURATION_MS,
}: CountUpProps) {
  const [shown, setShown] = useState(value);

  useEffect(() => {
    if (window.matchMedia(REDUCED_MOTION_QUERY).matches) {
      return;
    }
    let frame = 0;
    const start = performance.now() + delayMs;
    const tick = (now: number): void => {
      const progress = Math.min(Math.max((now - start) / durationMs, 0), 1);
      setShown(value * easeOutCubic(progress));
      if (progress < 1) {
        frame = window.requestAnimationFrame(tick);
      }
    };
    frame = window.requestAnimationFrame(tick);
    return () => window.cancelAnimationFrame(frame);
  }, [value, delayMs, durationMs]);

  return (
    <>
      {new Intl.NumberFormat(LOCALE, {
        minimumFractionDigits: decimals,
        maximumFractionDigits: decimals,
      }).format(shown)}
    </>
  );
}
