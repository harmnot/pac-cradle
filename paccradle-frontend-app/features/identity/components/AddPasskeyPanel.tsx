"use client";

import { useRouter, useSearchParams } from "next/navigation";
import { startTransition, useEffect, useState } from "react";
import { StepTransition, TransitionType } from "@/components/motion/PageTransition";
import { Button, ButtonSize, ButtonVariant } from "@/components/ui/Button";
import { Icon } from "@/components/ui/Icon";
import { Notice, NoticeTone } from "@/components/ui/Notice";
import { describeError } from "@/lib/errors";
import { t } from "@/lib/i18n";
import { addPasskey, passkeyRegistrationOptions } from "../api";
import { canMakePasskeyHere, deviceName, makePasskey, PasskeyFailure } from "../passkey";
import { NEXT_PARAM, safeNextPath, skipPasskeyHere } from "../routes";
import { useSession } from "../session";
import { NextStep } from "../types";
import { AuthCard } from "./AuthShell";
import { RequireAccount, type ActiveSession } from "./RequireAccount";
import styles from "./AddPasskeyPanel.module.css";

const ERROR_ALREADY_REGISTERED = "AUTH_PASSKEY_ALREADY_REGISTERED";
/** How long the "saved" check shows before moving on. */
const DONE_PAUSE_MS = 1600;

/** Where the passkey screen stands. */
type Stage = "checking" | "unavailable" | "ready" | "working" | "done" | "already";

/** The passkey screen, for the signed-in account. */
export function AddPasskeyPanel() {
  const next = safeNextPath(useSearchParams().get(NEXT_PARAM));
  return <RequireAccount>{(session) => <PasskeyStep session={session} next={next} />}</RequireAccount>;
}

/**
 * Offers a passkey on this device (`next_steps: add_passkey`): Touch ID, Windows Hello or the
 * phone's lock instead of a password next time. A device that already holds one of the
 * account's passkeys counts as done. "Not now" asks again after the next sign-in; "Don't ask
 * on this device" remembers the answer here only.
 */
function PasskeyStep({ session, next }: { readonly session: ActiveSession; readonly next: string }) {
  const router = useRouter();
  const { completeStep, reloadAccount } = useSession();
  const [stage, setStage] = useState<Stage>("checking");
  const [notice, setNotice] = useState("");

  useEffect(() => {
    let live = true;
    const check = async () => {
      const capable = await canMakePasskeyHere();
      if (live) {
        startTransition(() => setStage(capable ? "ready" : "unavailable"));
      }
    };
    void check();
    return () => {
      live = false;
    };
  }, []);

  const goOn = () => {
    completeStep(NextStep.AddPasskey);
    router.replace(next, { transitionTypes: [TransitionType.Forward] });
  };

  const saved = async (stageReached: "done" | "already") => {
    startTransition(() => setStage(stageReached));
    await reloadAccount();
    setTimeout(goOn, DONE_PAUSE_MS);
  };

  const create = async () => {
    setStage("working");
    setNotice("");
    const options = await passkeyRegistrationOptions();
    if (options.kind === "error") {
      setStage("ready");
      setNotice(describeError(options.error));
      return;
    }
    const outcome = await makePasskey(options.value.public_key);
    if (outcome.kind === "failed") {
      switch (outcome.failure) {
        case PasskeyFailure.AlreadyOnDevice:
          await saved("already");
          return;
        case PasskeyFailure.Unsupported:
          startTransition(() => setStage("unavailable"));
          return;
        default:
          setStage("ready");
          setNotice(t("passkey.cancelled"));
          return;
      }
    }
    const result = await addPasskey({
      ceremonyId: options.value.ceremony_id,
      credential: outcome.credential,
      name: deviceName(),
    });
    if (result.kind === "ok") {
      await saved("done");
      return;
    }
    if (result.error.code === ERROR_ALREADY_REGISTERED) {
      await saved("already");
      return;
    }
    setStage("ready");
    setNotice(describeError(result.error));
  };

  const dontAskHere = () => {
    skipPasskeyHere(session.account);
    goOn();
  };

  const finished = stage === "done" || stage === "already";

  return (
    <AuthCard eyebrow={t("passkey.eyebrow")} title={t("passkey.title")} lead={t("passkey.lead")}>
      <StepTransition step={finished ? "finished" : stage === "unavailable" ? "unavailable" : "offer"}>
        {finished ? (
          <div className={styles.done} role="status">
            <svg viewBox="0 0 64 64" className={styles.check} aria-hidden="true">
              <circle cx="32" cy="32" r="28" pathLength={1} />
              <path d="m20 33 8 8 16-17" pathLength={1} />
            </svg>
            <p>{stage === "done" ? t("passkey.saved") : t("passkey.already")}</p>
          </div>
        ) : stage === "unavailable" ? (
          <div className={styles.offer}>
            <Notice>{t("passkey.unavailable")}</Notice>
            <Button size={ButtonSize.Large} fullWidth withArrow onClick={goOn}>
              {t("passkey.continue")}
            </Button>
          </div>
        ) : (
          <div className={styles.offer}>
            <ul className={styles.benefits}>
              <li>
                <Icon name="passkey" size={20} />
                {t("passkey.benefit.fast")}
              </li>
              <li>
                <Icon name="shield" size={20} />
                {t("passkey.benefit.safe")}
              </li>
              <li>
                <Icon name="device" size={20} />
                {t("passkey.benefit.device")}
              </li>
            </ul>
            {notice !== "" && <Notice tone={NoticeTone.Error}>{notice}</Notice>}
            <Button
              size={ButtonSize.Large}
              fullWidth
              withArrow
              onClick={create}
              busy={stage === "working" || stage === "checking"}
            >
              {t("passkey.create")}
            </Button>
            <div className={styles.later}>
              <Button variant={ButtonVariant.Quiet} onClick={goOn} disabled={stage === "working"}>
                {t("passkey.notNow")}
              </Button>
              <Button variant={ButtonVariant.Quiet} onClick={dontAskHere} disabled={stage === "working"}>
                {t("passkey.dontAsk")}
              </Button>
            </div>
          </div>
        )}
      </StepTransition>
    </AuthCard>
  );
}
