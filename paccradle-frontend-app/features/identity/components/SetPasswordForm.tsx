"use client";

import { useRouter, useSearchParams } from "next/navigation";
import { useState, type FormEvent } from "react";
import { TransitionType } from "@/components/motion/PageTransition";
import { Button, ButtonSize, ButtonVariant } from "@/components/ui/Button";
import { Notice, NoticeTone } from "@/components/ui/Notice";
import { TextField } from "@/components/ui/TextField";
import { describeError } from "@/lib/errors";
import { t, tCount, type MessageKey } from "@/lib/i18n";
import { none, some } from "@/lib/maybe";
import { setPassword } from "../api";
import { canMakePasskeyHere } from "../passkey";
import { NEXT_PARAM, routeAfterSignIn, safeNextPath } from "../routes";
import { useSession } from "../session";
import { NextStep, PASSWORD_MAX_LENGTH, PASSWORD_MIN_LENGTH } from "../types";
import { AuthCard } from "./AuthShell";
import { RequireAccount, type ActiveSession } from "./RequireAccount";
import styles from "./SetPasswordForm.module.css";

const ERROR_REAUTH = "AUTH_REAUTH_REQUIRED";
const ERROR_REJECTED = "AUTH_PASSWORD_REJECTED";
const ERROR_WRONG_CURRENT = "AUTH_INVALID_CREDENTIALS";

/** Lengths the meter fills at: the minimum, then a few steps beyond it. */
const METER_STEPS = [PASSWORD_MIN_LENGTH, 16, 20, 24] as const;
/** What each filled step says (index = level − 1). */
const METER_LABELS: readonly MessageKey[] = [
  "password.meter.level1",
  "password.meter.level2",
  "password.meter.level3",
  "password.meter.level4",
];

/** How far along the meter a password is, 0 to `METER_STEPS.length`. */
function meterLevel(length: number): number {
  return METER_STEPS.filter((step) => length >= step).length;
}

/** The password screen, for the signed-in account. */
export function SetPasswordForm() {
  const next = safeNextPath(useSearchParams().get(NEXT_PARAM));
  return <RequireAccount>{(session) => <PasswordStep session={session} next={next} />}</RequireAccount>;
}

/**
 * Sets a password (`next_steps: set_password`) or changes it. Within 15 minutes of a link or
 * passkey sign-in no current password is needed; otherwise the API asks for it
 * (`AUTH_REAUTH_REQUIRED`) and the field appears. Changing it signs out every other device.
 */
function PasswordStep({ session, next }: { readonly session: ActiveSession; readonly next: string }) {
  const router = useRouter();
  const { completeStep, reloadAccount } = useSession();
  const changing = session.account.hasPassword;
  const [password, setValue] = useState("");
  const [current, setCurrent] = useState("");
  const [needCurrent, setNeedCurrent] = useState(false);
  const [busy, setBusy] = useState(false);
  const [fieldError, setFieldError] = useState("");
  const [currentError, setCurrentError] = useState("");
  const [notice, setNotice] = useState("");
  const level = meterLevel(password.length);
  const short = Math.max(PASSWORD_MIN_LENGTH - password.length, 0);

  const goOn = async () => {
    completeStep(NextStep.SetPassword);
    const remaining = session.nextSteps.filter((step) => step !== NextStep.SetPassword);
    const canMakePasskey = await canMakePasskeyHere();
    router.replace(
      routeAfterSignIn({
        session: { account: { ...session.account, hasPassword: true }, nextSteps: remaining },
        next,
        canMakePasskey,
      }),
      { transitionTypes: [TransitionType.Forward] },
    );
  };

  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setBusy(true);
    setFieldError("");
    setCurrentError("");
    const result = await setPassword({
      password,
      currentPassword: needCurrent ? some(current) : none,
    });
    setBusy(false);
    if (result.kind === "ok") {
      await reloadAccount();
      await goOn();
      return;
    }
    switch (result.error.code) {
      case ERROR_REAUTH:
        setNeedCurrent(true);
        setNotice(describeError(result.error));
        return;
      case ERROR_REJECTED:
        setFieldError(describeError(result.error));
        return;
      case ERROR_WRONG_CURRENT:
        setCurrentError(t("password.currentWrong"));
        return;
      default:
        setNotice(describeError(result.error));
    }
  };

  return (
    <AuthCard
      eyebrow={changing ? t("password.change.eyebrow") : t("password.set.eyebrow")}
      title={changing ? t("password.change.title") : t("password.set.title")}
      lead={changing ? t("password.change.lead") : t("password.set.lead")}
    >
      {notice !== "" && <Notice tone={needCurrent ? NoticeTone.Info : NoticeTone.Error}>{notice}</Notice>}
      <form className={styles.form} onSubmit={submit}>
        {/* Lets password managers save the new password under the right account. */}
        <input
          type="email"
          name="username"
          autoComplete="username"
          value={session.account.email.kind === "some" ? session.account.email.value : ""}
          readOnly
          hidden
        />
        {needCurrent && (
          <TextField
            label={t("password.current.label")}
            type="password"
            name="current-password"
            autoComplete="current-password"
            required
            revealable
            value={current}
            onChange={(event) => setCurrent(event.target.value)}
            error={currentError || undefined}
          />
        )}
        <TextField
          label={changing ? t("password.new.label") : t("field.password.label")}
          type="password"
          name="new-password"
          autoComplete="new-password"
          required
          revealable
          minLength={PASSWORD_MIN_LENGTH}
          maxLength={PASSWORD_MAX_LENGTH}
          value={password}
          onChange={(event) => setValue(event.target.value)}
          error={fieldError || undefined}
        />
        <div className={styles.meter} data-level={level}>
          <span className={styles.bars} aria-hidden="true">
            {METER_STEPS.map((step) => (
              <span key={step} className={styles.bar} />
            ))}
          </span>
          <span className={styles.meterText} aria-live="polite">
            {short > 0 ? tCount({ base: "password.meter.short", count: short }) : t(METER_LABELS[level - 1])}
          </span>
        </div>
        <Button type="submit" size={ButtonSize.Large} fullWidth withArrow busy={busy} disabled={short > 0}>
          {changing ? t("password.change.submit") : t("password.set.submit")}
        </Button>
      </form>
      {!changing && (
        <Button variant={ButtonVariant.Quiet} onClick={goOn}>
          {t("password.skip")}
        </Button>
      )}
    </AuthCard>
  );
}
