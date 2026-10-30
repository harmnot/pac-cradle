"use client";

import { useRouter } from "next/navigation";
import { useEffect, useRef, useState } from "react";
import { TransitionType } from "@/components/motion/PageTransition";
import { LinkButton } from "@/components/ui/Button";
import { Notice, NoticeTone } from "@/components/ui/Notice";
import { describeError } from "@/lib/errors";
import { t } from "@/lib/i18n";
import { none, some, type Maybe } from "@/lib/maybe";
import { signInWithLink } from "../api";
import { canMakePasskeyHere } from "../passkey";
import { AppRoute, routeAfterSignIn } from "../routes";
import { useSession } from "../session";
import { AuthCard } from "./AuthShell";
import styles from "./LinkSignIn.module.css";

const TOKEN_PARAM = "token";

/** The token in `#token=…`, read once and wiped from the address bar and history. */
function takeToken(): Maybe<string> {
  const token = new URLSearchParams(window.location.hash.slice(1)).get(TOKEN_PARAM);
  window.history.replaceState(window.history.state, "", window.location.pathname + window.location.search);
  return token === null || token === "" ? none : some(token);
}

/**
 * The page an emailed link opens (`{APP_WEB_URL}/auth/link#token=…`). The token rides in the
 * fragment, so it never reaches a server log; this page reads it, clears it from the address
 * bar, and signs in, which confirms the address on sign-up.
 */
export function LinkSignIn() {
  const router = useRouter();
  const { signedIn } = useSession();
  const [problem, setProblem] = useState("");
  const started = useRef(false);

  useEffect(() => {
    // The token is good once; a second run (React's development double effect) must not
    // find the wiped address and report a broken link.
    if (started.current) {
      return;
    }
    started.current = true;
    const signIn = async (token: Maybe<string>) => {
      if (token.kind === "none") {
        setProblem(t("link.missing"));
        return;
      }
      const result = await signInWithLink(token.value);
      if (result.kind === "error") {
        setProblem(describeError(result.error));
        return;
      }
      signedIn(result.value);
      const canMakePasskey = await canMakePasskeyHere();
      router.replace(
        routeAfterSignIn({ session: result.value, next: AppRoute.Workspace, canMakePasskey }),
        { transitionTypes: [TransitionType.Forward] },
      );
    };
    void signIn(takeToken());
  }, [router, signedIn]);

  if (problem !== "") {
    return (
      <AuthCard eyebrow={t("link.eyebrow")} title={t("link.failed.title")}>
        <Notice tone={NoticeTone.Error}>{problem}</Notice>
        <LinkButton href={AppRoute.SignIn} withArrow>
          {t("link.failed.action")}
        </LinkButton>
      </AuthCard>
    );
  }

  return (
    <AuthCard eyebrow={t("link.eyebrow")} title={t("link.working.title")} lead={t("link.working.lead")}>
      <div className={styles.working} role="status" aria-live="polite">
        <span className={styles.orbit} aria-hidden="true">
          <span />
          <span />
          <span />
        </span>
        <span className="visually-hidden">{t("link.working.title")}</span>
      </div>
    </AuthCard>
  );
}
