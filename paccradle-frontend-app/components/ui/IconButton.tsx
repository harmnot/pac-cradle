import type { ButtonHTMLAttributes } from "react";
import { Icon, type IconName } from "./Icon";
import styles from "./IconButton.module.css";

/** Props for an icon button. */
export type IconButtonProps = {
  readonly icon: IconName;
  /** What it does: its accessible name, and the label that rises above it on hover or focus. */
  readonly label: string;
  /** For toggles: whether it is on (`aria-pressed`). */
  readonly pressed?: boolean;
} & Omit<ButtonHTMLAttributes<HTMLButtonElement>, "className" | "children" | "aria-label" | "aria-pressed">;

/**
 * A round icon-only button for toolbars (undo, redo, remove…): its label rises above it as a
 * cream tag on hover and keyboard focus. Defaults to `type="button"`.
 */
export function IconButton({ icon, label, pressed, type = "button", ...rest }: IconButtonProps) {
  return (
    <button type={type} className={styles.button} aria-label={label} aria-pressed={pressed} {...rest}>
      <Icon name={icon} size={20} />
      <span className={styles.tip} aria-hidden="true">
        {label}
      </span>
    </button>
  );
}
