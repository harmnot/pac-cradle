import type { ReactNode } from "react";
import { onRadioKey } from "./radio-keys";
import styles from "./ChoiceChips.module.css";

/** One chip of a choice. */
export interface ChoiceChip<T extends string | number> {
  readonly value: T;
  readonly label: string;
  /** A small drawing before the label. */
  readonly art?: ReactNode;
}

/** Props for a row of choice chips. */
export interface ChoiceChipsProps<T extends string | number> {
  /** The group's name, read by screen readers. */
  readonly label: string;
  readonly options: readonly ChoiceChip<T>[];
  /** The chosen value; none chosen when no option has it (a typed quantity off the presets). */
  readonly value: T;
  readonly onChange: (value: T) => void;
  readonly disabled?: boolean;
}

/**
 * A radio group of pills (quick counts, grid layouts, box boards): the chosen one fills sage.
 * Arrow keys move the choice (`radio-keys.ts`).
 */
export function ChoiceChips<T extends string | number>({ label, options, value, onChange, disabled = false }: ChoiceChipsProps<T>) {
  const values = options.map((option) => option.value);
  const anyChecked = values.includes(value);
  return (
    <div
      role="radiogroup"
      aria-label={label}
      className={styles.chips}
      onKeyDown={(event) => onRadioKey({ event, values, value, onChange })}
    >
      {options.map((option, index) => {
        const checked = option.value === value;
        return (
          <button
            key={option.value}
            type="button"
            role="radio"
            aria-checked={checked}
            tabIndex={checked || (!anyChecked && index === 0) ? 0 : -1}
            disabled={disabled}
            className={styles.chip}
            onClick={() => onChange(option.value)}
          >
            {option.art}
            {option.label}
          </button>
        );
      })}
    </div>
  );
}
