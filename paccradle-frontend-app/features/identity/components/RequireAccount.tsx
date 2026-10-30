"use client";

import { usePathname, useRouter } from "next/navigation";
import { useEffect, type ReactNode } from "react";
import { Button, ButtonVariant } from "@/components/ui/Button";
import { Notice, NoticeTone } from "@/components/ui/Notice";
import { Skeleton, SkeletonShape } from "@/components/ui/Skeleton";
import { t } from "@/lib/i18n";
import { AppRoute, withNext } from "../routes";
import { isAccount, useSession, type SessionState } from "../session";
import styles from "./RequireAccount.module.css";

/** A session that belongs to a confirmed account. */
export type ActiveSession = Extract<SessionState, { status: "active" }>;

/** Tells the person the API can't be reached, with a retry. */
export function SessionUnreachable() {
  const { retry } = useSession();
  return (
    <Notice
      tone={NoticeTone.Error}
      action={
        <Button variant={ButtonVariant.Secondary} onClick={retry}>
          {t("session.retry")}
        </Button>
      }
    >
      {t("session.unreachable")}
    </Notice>
  );
}

/**
 * Shows `children` to a signed-in account. A visitor or a guest goes to sign-in, coming back
 * here afterwards (`?next=`). The session lives in the browser, so this check runs there.
 */
export function RequireAccount({ children }: { readonly children: (session: ActiveSession) => ReactNode }) {
  const { state } = useSession();
  const router = useRouter();
  const pathname = usePathname();
  const mustSignIn = state.status === "none" || (state.status === "active" && !isAccount(state));

  useEffect(() => {
    if (mustSignIn) {
      router.replace(withNext({ route: AppRoute.SignIn, next: pathname }));
    }
  }, [mustSignIn, pathname, router]);

  if (state.status === "unreachable") {
    return <SessionUnreachable />;
  }
  if (!isAccount(state)) {
    return (
      <div className={styles.waiting} aria-busy="true">
        <Skeleton shape={SkeletonShape.Title} />
        <Skeleton count={3} />
        <span className="visually-hidden">{t("session.checking")}</span>
      </div>
    );
  }
  return children(state);
}
