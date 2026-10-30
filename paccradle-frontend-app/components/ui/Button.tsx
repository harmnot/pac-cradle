import Link from "next/link";
import type { ButtonHTMLAttributes, ReactNode } from "react";
import { Icon } from "./Icon";
import styles from "./Button.module.css";

/** Visual styles a button can have. */
export const ButtonVariant = {
  /** Deep sage: the main action. */
  Primary: "primary",
  /** Outlined on paper. */
  Secondary: "secondary",
  /** Terracotta: the main action on dark or sage panels. */
  Accent: "accent",
  /** Text only, underlined on hover: the quiet way out ("Use your password instead"). */
  Quiet: "quiet",
} as const;

export type ButtonVariant = (typeof ButtonVariant)[keyof typeof ButtonVariant];

/** Button sizes. */
export const ButtonSize = {
  Medium: "medium",
  Large: "large",
} as const;

export type ButtonSize = (typeof ButtonSize)[keyof typeof ButtonSize];

interface CommonProps {
  readonly variant?: ButtonVariant;
  readonly size?: ButtonSize;
  /** Adds an arrow that slides forward on hover. */
  readonly withArrow?: boolean;
  /** Fills its container's width (form actions). */
  readonly fullWidth?: boolean;
  readonly children: ReactNode;
}

/** Props for a link styled as a button. */
export interface LinkButtonProps extends CommonProps {
  readonly href: string;
  /** The page transition to play (`TransitionType` in components/motion/PageTransition). */
  readonly transitionTypes?: readonly string[];
}

/** Props for a real `<button>`. */
export type ActionButtonProps = CommonProps & {
  /** Working on it: shows a spinner, sets `aria-busy` and ignores further clicks. */
  readonly busy?: boolean;
} & Omit<ButtonHTMLAttributes<HTMLButtonElement>, "className" | "children">;

interface ClassOptions {
  readonly variant: ButtonVariant;
  readonly size: ButtonSize;
  readonly fullWidth: boolean;
}

function classFor({ variant, size, fullWidth }: ClassOptions): string {
  return `${styles.button} ${styles[variant]} ${styles[size]} ${fullWidth ? styles.fullWidth : ""}`;
}

function Content({
  withArrow,
  busy = false,
  children,
}: Pick<CommonProps, "withArrow" | "children"> & { readonly busy?: boolean }) {
  return (
    <>
      {busy && <span className={styles.spinner} aria-hidden="true" />}
      <span className={styles.label}>{children}</span>
      {withArrow && !busy && <Icon name="arrow" size={18} className={styles.arrow} />}
    </>
  );
}

/** A navigation link that looks like a button: lifts, shines and nudges its arrow on hover. */
export function LinkButton({
  href,
  variant = ButtonVariant.Primary,
  size = ButtonSize.Medium,
  withArrow = false,
  fullWidth = false,
  transitionTypes,
  children,
}: LinkButtonProps) {
  return (
    <Link
      href={href}
      className={classFor({ variant, size, fullWidth })}
      transitionTypes={transitionTypes === undefined ? undefined : [...transitionTypes]}
    >
      <Content withArrow={withArrow}>{children}</Content>
    </Link>
  );
}

/** A button for actions on the current page. Defaults to `type="button"`. */
export function Button({
  variant = ButtonVariant.Primary,
  size = ButtonSize.Medium,
  withArrow = false,
  fullWidth = false,
  busy = false,
  children,
  type = "button",
  disabled,
  ...rest
}: ActionButtonProps) {
  return (
    <button
      type={type}
      className={classFor({ variant, size, fullWidth })}
      aria-busy={busy}
      disabled={disabled || busy}
      {...rest}
    >
      <Content withArrow={withArrow} busy={busy}>
        {children}
      </Content>
    </button>
  );
}
