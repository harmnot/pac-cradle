import type { KeyboardEvent } from "react";

/**
 * Arrow keys for a radio group built from buttons (`role="radio"`, WAI-ARIA radio group):
 * arrows move the choice to the next or previous option and focus it, Home and End jump to
 * the ends. Only the chosen option is in the tab order.
 */

const FORWARD = new Set(["ArrowRight", "ArrowDown"]);
const BACKWARD = new Set(["ArrowLeft", "ArrowUp"]);

/** The option a key moves to, or `undefined` for keys that don't move. */
function target<T>({ key, values, current }: { key: string; values: readonly T[]; current: number }): T | undefined {
  const last = values.length - 1;
  if (FORWARD.has(key)) {
    return values[current >= last ? 0 : current + 1];
  }
  if (BACKWARD.has(key)) {
    return values[current <= 0 ? last : current - 1];
  }
  if (key === "Home") {
    return values[0];
  }
  return key === "End" ? values[last] : undefined;
}

/**
 * Handles a key on the group: chooses the option it moves to and focuses its button. It steps
 * from the focused option, not `value`: a choice made in a transition may not have rendered
 * yet when the next key comes.
 */
export function onRadioKey<T>({
  event,
  values,
  value,
  onChange,
}: {
  event: KeyboardEvent<HTMLElement>;
  values: readonly T[];
  value: T;
  onChange: (next: T) => void;
}): void {
  const buttons = [...event.currentTarget.querySelectorAll<HTMLElement>('[role="radio"]')];
  const focused = buttons.findIndex((button) => button === event.target);
  const next = target({ key: event.key, values, current: focused >= 0 ? focused : values.indexOf(value) });
  if (next === undefined) {
    return;
  }
  event.preventDefault();
  onChange(next);
  buttons[values.indexOf(next)]?.focus();
}
