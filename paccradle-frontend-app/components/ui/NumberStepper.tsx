"use client";

import { useId, useState, type ChangeEvent, type KeyboardEvent } from "react";
import { IconButton } from "./IconButton";
import styles from "./NumberStepper.module.css";

/** Props for a whole-number stepper. */
export interface NumberStepperProps {
  readonly label: string;
  readonly value: number;
  readonly min: number;
  readonly max: number;
  readonly onChange: (value: number) => void;
  /** The − and + buttons' names ("One fewer", "One more"). */
  readonly decreaseLabel: string;
  readonly increaseLabel: string;
  /** Words after the number, e.g. "products". */
  readonly unit?: string;
}

/**
 * A whole number between − and + buttons, typed straight into the middle too. The number
 * ticks up or down as it changes; arrow keys step it, Page Up/Down by ten.
 */
export function NumberStepper({ label, value, min, max, onChange, decreaseLabel, increaseLabel, unit }: NumberStepperProps) {
  const id = useId();
  // While typing, the field may hold what isn't a number yet ("" or "1e"); it settles on blur.
  const [draft, setDraft] = useState<{ readonly text: string; readonly typing: boolean }>({ text: "", typing: false });
  const BIG_STEP = 10;

  const set = (next: number) => onChange(Math.min(max, Math.max(min, Math.round(next))));

  const type = (event: ChangeEvent<HTMLInputElement>) => {
    const text = event.target.value;
    setDraft({ text, typing: true });
    const parsed = Number.parseInt(text, 10);
    if (Number.isFinite(parsed) && parsed >= min && parsed <= max) {
      onChange(parsed);
    }
  };

  const keys = (event: KeyboardEvent<HTMLInputElement>) => {
    const steps: Readonly<Record<string, number>> = { PageUp: BIG_STEP, PageDown: -BIG_STEP };
    const step = steps[event.key];
    if (step !== undefined) {
      event.preventDefault();
      set(value + step);
    }
  };

  return (
    <div className={styles.stepper}>
      <label htmlFor={id} className={styles.label}>
        {label}
      </label>
      <div className={styles.control}>
        <IconButton icon="minus" label={decreaseLabel} onClick={() => set(value - 1)} disabled={value <= min} />
        <span className={styles.field}>
          <input
            id={id}
            className={styles.input}
            type="number"
            inputMode="numeric"
            min={min}
            max={max}
            step={1}
            value={draft.typing ? draft.text : String(value)}
            onChange={type}
            onKeyDown={keys}
            onBlur={() => setDraft({ text: "", typing: false })}
          />
          <span key={value} className={styles.tick} aria-hidden="true" />
        </span>
        <IconButton icon="plus" label={increaseLabel} onClick={() => set(value + 1)} disabled={value >= max} />
        {unit !== undefined && <span className={styles.unit}>{unit}</span>}
      </div>
    </div>
  );
}
