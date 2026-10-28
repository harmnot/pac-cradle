"use client";

import { useActionState, useId } from "react";
import { Button, ButtonSize, ButtonVariant } from "@/components/ui/Button";
import { AnalyticsEvent, trackEvent } from "@/lib/analytics";
import { t } from "@/lib/i18n";
import { joinWaitlist } from "../api";
import { WaitlistTone, type WaitlistErrorReason, type WaitlistSource } from "../types";
import styles from "./WaitlistForm.module.css";

type FormState =
  | { readonly kind: "idle"; readonly email: string }
  | { readonly kind: "ok" }
  | { readonly kind: "error"; readonly reason: WaitlistErrorReason; readonly email: string };

const INITIAL_STATE: FormState = { kind: "idle", email: "" };
const EMAIL_FIELD = "email";

/** Check mark drawn in when a sign-up succeeds. */
const CHECK_PATH = "m7 12.5 3.5 3.5L17 9";

/** Props for the waitlist form. */
export interface WaitlistFormProps {
  readonly source: WaitlistSource;
  readonly tone?: WaitlistTone;
}

/**
 * Email sign-up form for the beta waitlist. Uses native validation and a React form action;
 * a successful sign-up swaps the form for a confirmation with a drawn check mark.
 */
export function WaitlistForm({ source, tone = WaitlistTone.Light }: WaitlistFormProps) {
  const inputId = useId();
  const messageId = useId();

  const submit = async (_previous: FormState, formData: FormData): Promise<FormState> => {
    const email = String(formData.get(EMAIL_FIELD) ?? "").trim();
    const result = await joinWaitlist({ email, source });
    if (result.kind === "error") {
      return { kind: "error", reason: result.reason, email };
    }
    trackEvent(AnalyticsEvent.WaitlistSignup, { source });
    return { kind: "ok" };
  };

  const [state, formAction, pending] = useActionState(submit, INITIAL_STATE);
  const isDark = tone === WaitlistTone.Dark;
  const toneClass = isDark ? styles.dark : "";

  if (state.kind === "ok") {
    return (
      <p role="status" className={`${styles.success} ${toneClass}`}>
        <svg viewBox="0 0 24 24" aria-hidden="true" className={styles.successIcon}>
          <circle cx="12" cy="12" r="10" />
          <path pathLength={1} d={CHECK_PATH} />
        </svg>
        {t("waitlist.success")}
      </p>
    );
  }

  return (
    <form
      action={formAction}
      className={`${styles.form} ${toneClass}`}
      aria-describedby={messageId}
    >
      <label htmlFor={inputId} className="visually-hidden">
        {t("waitlist.label")}
      </label>
      <input
        id={inputId}
        name={EMAIL_FIELD}
        type="email"
        required
        autoComplete="email"
        defaultValue={state.email}
        placeholder={t("waitlist.placeholder")}
        className={styles.input}
      />
      <Button
        type="submit"
        disabled={pending}
        size={ButtonSize.Large}
        variant={isDark ? ButtonVariant.Accent : ButtonVariant.Primary}
        withArrow={!pending}
      >
        {pending ? t("waitlist.submitting") : t("waitlist.submit")}
      </Button>
      <p id={messageId} role="status" aria-live="polite" className={styles.error}>
        {state.kind === "error" ? t(`waitlist.error.${state.reason}`) : ""}
      </p>
    </form>
  );
}
