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
  readonly children: ReactNode;
}

/** Props for a link styled as a button. */
export interface LinkButtonProps extends CommonProps {
  readonly href: string;
}

/** Props for a real `<button>`. */
export type ActionButtonProps = CommonProps &
  Omit<ButtonHTMLAttributes<HTMLButtonElement>, "className" | "children">;

interface ClassOptions {
  readonly variant: ButtonVariant;
  readonly size: ButtonSize;
}

function classFor({ variant, size }: ClassOptions): string {
  return `${styles.button} ${styles[variant]} ${styles[size]}`;
}

function Content({ withArrow, children }: Pick<CommonProps, "withArrow" | "children">) {
  return (
    <>
      <span className={styles.label}>{children}</span>
      {withArrow && <Icon name="arrow" size={18} className={styles.arrow} />}
    </>
  );
}

/** A navigation link that looks like a button: lifts, shines and nudges its arrow on hover. */
export function LinkButton({
  href,
  variant = ButtonVariant.Primary,
  size = ButtonSize.Medium,
  withArrow = false,
  children,
}: LinkButtonProps) {
  return (
    <Link href={href} className={classFor({ variant, size })}>
      <Content withArrow={withArrow}>{children}</Content>
    </Link>
  );
}

/** A button for actions on the current page. Defaults to `type="button"`. */
export function Button({
  variant = ButtonVariant.Primary,
  size = ButtonSize.Medium,
  withArrow = false,
  children,
  type = "button",
  ...rest
}: ActionButtonProps) {
  return (
    <button type={type} className={classFor({ variant, size })} {...rest}>
      <Content withArrow={withArrow}>{children}</Content>
    </button>
  );
}
