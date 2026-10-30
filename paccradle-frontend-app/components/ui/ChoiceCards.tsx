import type { ReactNode } from "react";
import { Icon } from "./Icon";
import { onRadioKey } from "./radio-keys";
import styles from "./ChoiceCards.module.css";

/** One card of a choice. */
export interface ChoiceCard<T extends string> {
  readonly value: T;
  readonly title: string;
  readonly body?: string;
  /** A small measurement line in mono, e.g. "≈ 1.6 mm". */
  readonly meta?: string;
  /** A tag on the card's corner, e.g. "Recommended". */
  readonly badge?: string;
  /** A drawing above the title. */
  readonly art?: ReactNode;
}

/** Props for a group of choice cards. */
export interface ChoiceCardsProps<T extends string> {
  /** The id of the heading that names the group. */
  readonly labelledBy: string;
  readonly options: readonly ChoiceCard<T>[];
  readonly value: T;
  readonly onChange: (value: T) => void;
  /** `wide` cards hold a drawing (styles, boxes); `compact` ones a short line of art (boards). */
  readonly size?: "wide" | "compact";
  readonly disabled?: boolean;
}

/**
 * A radio group of paper cards: each lifts and warms on hover, the chosen one rises with a
 * sage ring and a tick. Arrow keys move the choice (`radio-keys.ts`).
 */
export function ChoiceCards<T extends string>({ labelledBy, options, value, onChange, size = "wide", disabled = false }: ChoiceCardsProps<T>) {
  const values = options.map((option) => option.value);
  return (
    <div
      role="radiogroup"
      aria-labelledby={labelledBy}
      aria-disabled={disabled || undefined}
      className={`${styles.cards} ${styles[size]}`}
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
            tabIndex={checked ? 0 : -1}
            disabled={disabled}
            className={styles.card}
            style={{ animationDelay: `${index * 60}ms` }}
            onClick={() => onChange(option.value)}
          >
            {option.badge !== undefined && (
              <span className={styles.badge}>
                <Icon name="sparkle" size={12} />
                {option.badge}
              </span>
            )}
            {option.art !== undefined && <span className={styles.art}>{option.art}</span>}
            <span className={styles.title}>{option.title}</span>
            {option.meta !== undefined && <span className={styles.meta}>{option.meta}</span>}
            {option.body !== undefined && <span className={styles.body}>{option.body}</span>}
            <span className={styles.check} aria-hidden="true">
              <Icon name="check" size={14} />
            </span>
          </button>
        );
      })}
    </div>
  );
}
