import type { ReactNode } from "react";
import { Icon, type IconName } from "./Icon";
import styles from "./Notice.module.css";

/** What a notice tells. */
export const NoticeTone = {
  Info: "info",
  Success: "success",
  Error: "error",
} as const;

export type NoticeTone = (typeof NoticeTone)[keyof typeof NoticeTone];

const ICON_BY_TONE: Readonly<Record<NoticeTone, IconName>> = {
  info: "sparkle",
  success: "check",
  error: "alert",
};

/** Props for a notice. */
export interface NoticeProps {
  readonly tone?: NoticeTone;
  readonly children: ReactNode;
  /** An action at the end, e.g. "Try again". */
  readonly action?: ReactNode;
}

/**
 * A short message on a tinted label band that slides in. Errors are announced at once
 * (`role="alert"`), the rest politely (`role="status"`).
 */
export function Notice({ tone = NoticeTone.Info, children, action }: NoticeProps) {
  return (
    <div className={`${styles.notice} ${styles[tone]}`} role={tone === NoticeTone.Error ? "alert" : "status"}>
      <Icon name={ICON_BY_TONE[tone]} size={18} className={styles.icon} />
      <p className={styles.text}>{children}</p>
      {action && <div className={styles.action}>{action}</div>}
    </div>
  );
}
