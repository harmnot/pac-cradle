"use client";

import { useEffect, useState } from "react";
import { Button, ButtonVariant } from "@/components/ui/Button";
import { Notice, NoticeTone } from "@/components/ui/Notice";
import { t } from "@/lib/i18n";
import styles from "./InboxSent.module.css";

/** Seconds before "Send it again" works (the API also allows 3 links per 10 minutes). */
const RESEND_AFTER_S = 30;
const SECOND_MS = 1000;
/** Mailpit's inbox from `scripts/dev-mail.sh`, shown in development only. */
const DEV_INBOX_URL = "http://localhost:8025";

/** Props for the inbox screen. */
export interface InboxSentProps {
  readonly email: string;
  /** Sends the link again; resolves to an error message, or "" when it went. */
  readonly onResend: () => Promise<string>;
  /** Back to the form, to use another address. */
  readonly onChangeEmail: () => void;
}

/**
 * "Check your inbox": an envelope seals itself, the address is repeated, and the link can be
 * sent again after a short wait. It never says whether an account exists (the API doesn't).
 */
export function InboxSent({ email, onResend, onChangeEmail }: InboxSentProps) {
  const [wait, setWait] = useState(RESEND_AFTER_S);
  const [sending, setSending] = useState(false);
  const [message, setMessage] = useState<{ tone: NoticeTone; text: string }>({
    tone: NoticeTone.Info,
    text: "",
  });

  useEffect(() => {
    if (wait === 0) {
      return;
    }
    const timer = setTimeout(() => setWait((seconds) => seconds - 1), SECOND_MS);
    return () => clearTimeout(timer);
  }, [wait]);

  const resend = async () => {
    setSending(true);
    const error = await onResend();
    setSending(false);
    setMessage(
      error === ""
        ? { tone: NoticeTone.Success, text: t("inbox.resent") }
        : { tone: NoticeTone.Error, text: error },
    );
    setWait(RESEND_AFTER_S);
  };

  return (
    <div className={styles.inbox}>
      <svg viewBox="0 0 120 96" className={styles.envelope} aria-hidden="true">
        <path className={styles.flap} d="M8 38 60 4l52 34" />
        <g className={styles.letter}>
          <rect className={styles.paper} x="22" y="8" width="76" height="58" rx="5" />
          <path className={styles.lines} d="M34 24h52M34 34h40M34 44h46" />
        </g>
        <path className={styles.body} d="M8 38 60 72l52-34v48a6 6 0 0 1-6 6H14a6 6 0 0 1-6-6Z" />
        <circle className={styles.seal} cx="60" cy="58" r="9" />
        <path className={styles.check} d="m55.5 58 3 3 6-6" pathLength={1} />
      </svg>
      <p className={styles.text}>{t("inbox.sentTo")}</p>
      <p className={styles.email}>{email}</p>
      <p className={styles.hint}>{t("inbox.hint")}</p>
      {message.text !== "" && <Notice tone={message.tone}>{message.text}</Notice>}
      {process.env.NODE_ENV === "development" && (
        <Notice>
          {t("inbox.devMailpit")}{" "}
          <a href={DEV_INBOX_URL} target="_blank" rel="noreferrer">
            {DEV_INBOX_URL}
          </a>
        </Notice>
      )}
      <div className={styles.actions}>
        <Button variant={ButtonVariant.Secondary} onClick={resend} busy={sending} disabled={wait > 0}>
          {wait > 0 ? t("inbox.resendIn", { seconds: wait }) : t("inbox.resend")}
        </Button>
        <Button variant={ButtonVariant.Quiet} onClick={onChangeEmail}>
          {t("inbox.changeEmail")}
        </Button>
      </div>
    </div>
  );
}
