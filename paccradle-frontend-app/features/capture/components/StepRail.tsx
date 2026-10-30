import type { CSSProperties } from "react";
import { t } from "@/lib/i18n";
import styles from "./StepRail.module.css";

/** Props for the step rail. */
export interface StepRailProps {
  /** The steps' names, in order. */
  readonly steps: readonly string[];
  /** Which one is current (0-based). */
  readonly current: number;
}

/**
 * Where the person is in the capture: dots on a line that fills as they go, the current dot
 * ringed. An ordered list, so screen readers hear "step 2 of 4".
 */
export function StepRail({ steps, current }: StepRailProps) {
  const progress = steps.length > 1 ? current / (steps.length - 1) : 1;
  return (
    <ol
      className={styles.rail}
      aria-label={t("capture.rail.label")}
      style={{ "--progress": progress } as CSSProperties}
    >
      {steps.map((step, index) => (
        <li
          key={step}
          className={styles.step}
          data-state={index < current ? "done" : index === current ? "current" : "next"}
          aria-current={index === current ? "step" : undefined}
        >
          <span className={styles.dot} aria-hidden="true" />
          <span className={styles.name}>{step}</span>
        </li>
      ))}
    </ol>
  );
}
