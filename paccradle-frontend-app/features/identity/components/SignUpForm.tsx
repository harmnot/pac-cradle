"use client";

import Link from "next/link";
import { useRouter, useSearchParams } from "next/navigation";
import { startTransition, useEffect, useState, type FormEvent } from "react";
import { StepTransition, TransitionType } from "@/components/motion/PageTransition";
import { Button, ButtonSize } from "@/components/ui/Button";
import { Notice, NoticeTone } from "@/components/ui/Notice";
import { TextField } from "@/components/ui/TextField";
import { describeError } from "@/lib/errors";
import { t } from "@/lib/i18n";
import { requestEmailLink } from "../api";
import { AppRoute, NEXT_PARAM, safeNextPath, withNext } from "../routes";
import { isAccount, useSession } from "../session";
import { AuthCard } from "./AuthShell";
import { InboxSent } from "./InboxSent";
import styles from "./SignInFlow.module.css";

/**
 * Sign-up: an email address, then "check your inbox". The emailed link confirms the address
 * and signs in; the password and a passkey come right after (MILESTONES M1-D3). A guest's
 * captures and designs move into the new account, since the link opens in their browser.
 */
export function SignUpForm() {
  const router = useRouter();
  const next = safeNextPath(useSearchParams().get(NEXT_PARAM));
  const { state: session } = useSession();
  const [email, setEmail] = useState("");
  const [sent, setSent] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    if (isAccount(session)) {
      router.replace(next);
    }
  }, [session, next, router]);

  const send = async (): Promise<string> => {
    const result = await requestEmailLink(email.trim());
    return result.kind === "ok" ? "" : describeError(result.error);
  };

  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setBusy(true);
    const problem = await send();
    setBusy(false);
    setError(problem);
    if (problem === "") {
      startTransition(() => setSent(true));
    }
  };

  return (
    <StepTransition step={sent ? "sent" : "form"}>
      {sent ? (
        <AuthCard eyebrow={t("signUp.eyebrow")} title={t("inbox.title")}>
          <InboxSent
            email={email.trim()}
            onResend={send}
            onChangeEmail={() => startTransition(() => setSent(false))}
          />
        </AuthCard>
      ) : (
        <AuthCard eyebrow={t("signUp.eyebrow")} title={t("signUp.title")} lead={t("signUp.lead")}>
          {error !== "" && <Notice tone={NoticeTone.Error}>{error}</Notice>}
          <form className={styles.form} onSubmit={submit}>
            <TextField
              label={t("field.email.label")}
              type="email"
              name="email"
              autoComplete="email"
              required
              value={email}
              onChange={(event) => setEmail(event.target.value)}
              placeholder={t("field.email.placeholder")}
              hint={t("signUp.email.hint")}
            />
            <Button type="submit" size={ButtonSize.Large} fullWidth withArrow busy={busy}>
              {t("signUp.submit")}
            </Button>
          </form>
          <p className={styles.switch}>
            {t("signUp.haveAccount")}{" "}
            <Link href={withNext({ route: AppRoute.SignIn, next })} transitionTypes={[TransitionType.Back]}>
              {t("signUp.signIn")}
            </Link>
          </p>
        </AuthCard>
      )}
    </StepTransition>
  );
}
