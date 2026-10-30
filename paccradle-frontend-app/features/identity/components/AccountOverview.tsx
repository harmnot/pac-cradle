"use client";

import { useRouter } from "next/navigation";
import { useState } from "react";
import { CountUp } from "@/components/motion/CountUp";
import { TransitionType } from "@/components/motion/PageTransition";
import { Button, ButtonVariant, LinkButton } from "@/components/ui/Button";
import { Icon } from "@/components/ui/Icon";
import { Notice, NoticeTone } from "@/components/ui/Notice";
import { Skeleton, SkeletonShape } from "@/components/ui/Skeleton";
import { describeError } from "@/lib/errors";
import { formatTimestamp, t, tCount } from "@/lib/i18n";
import { useApiQuery } from "@/lib/use-api";
import { fetchMe } from "../api";
import { AppRoute, withNext } from "../routes";
import { useSession } from "../session";
import { AccountRole, type Billing } from "../types";
import { initialOf } from "./AppHeader";
import { RequireAccount, type ActiveSession } from "./RequireAccount";
import styles from "./AccountOverview.module.css";

/** The account page, for the signed-in account. */
export function AccountOverview() {
  return <RequireAccount>{(session) => <Overview session={session} />}</RequireAccount>;
}

/** Plan, credits and the subscription, from `GET /me`. */
function PlanCard({ billing }: { readonly billing: Billing }) {
  const subscription = billing.subscription;
  return (
    <section className={styles.card} aria-labelledby="plan-title">
      <header className={styles.cardHead}>
        <Icon name="leaf" size={20} />
        <h2 id="plan-title">{t("account.plan.title")}</h2>
      </header>
      <p className={styles.plan}>{t(`plan.${billing.plan}`)}</p>
      <dl className={styles.facts}>
        <div>
          <dt>{t("account.plan.credits")}</dt>
          <dd>
            <CountUp value={billing.credits} durationMs={900} />
          </dd>
        </div>
        <div>
          <dt>{t("account.plan.unlocked")}</dt>
          <dd>
            <CountUp value={billing.unlockedDesigns} durationMs={900} />
          </dd>
        </div>
      </dl>
      {subscription.kind === "some" && (
        <p className={styles.note}>
          {subscription.value.currentPeriodEnd.kind === "some"
            ? t("account.plan.renews", {
                status: subscription.value.status,
                date: formatTimestamp(subscription.value.currentPeriodEnd.value),
              })
            : t("account.plan.status", { status: subscription.value.status })}
        </p>
      )}
    </section>
  );
}

function Overview({ session }: { readonly session: ActiveSession }) {
  const router = useRouter();
  const { signOut } = useSession();
  const me = useApiQuery({ key: "/me", load: (signal) => fetchMe(signal) });
  const [leaving, setLeaving] = useState(false);
  const { account } = session;
  const email = account.email.kind === "some" ? account.email.value : "";

  const leave = async () => {
    setLeaving(true);
    await signOut();
    router.replace(AppRoute.SignIn, { transitionTypes: [TransitionType.Back] });
  };

  return (
    <div className={styles.page}>
      <header className={styles.profile}>
        <span className={styles.avatar} aria-hidden="true">
          {initialOf(account)}
        </span>
        <div className={styles.who}>
          <p className={styles.eyebrow}>{t("account.eyebrow")}</p>
          <h1 className={styles.email}>{email}</h1>
          <p className={styles.since}>
            {t("account.since", { date: formatTimestamp(account.createdAt) })}
            {account.role === AccountRole.Admin && <span className={styles.badge}>{t("account.role.admin")}</span>}
          </p>
        </div>
      </header>

      <div className={styles.grid}>
        {me.value.kind === "some" ? (
          <PlanCard billing={me.value.value.billing} />
        ) : me.status === "error" && me.error.kind === "some" ? (
          <Notice tone={NoticeTone.Error}>{describeError(me.error.value)}</Notice>
        ) : (
          <Skeleton shape={SkeletonShape.Block} />
        )}

        <section className={styles.card} aria-labelledby="security-title">
          <header className={styles.cardHead}>
            <Icon name="shield" size={20} />
            <h2 id="security-title">{t("account.security.title")}</h2>
          </header>
          <ul className={styles.rows}>
            <li className={styles.row}>
              <Icon name="lock" size={20} className={styles.rowIcon} />
              <span className={styles.rowText}>
                <strong>{t("account.security.password")}</strong>
                <span>{account.hasPassword ? t("account.security.passwordSet") : t("account.security.passwordNone")}</span>
              </span>
              <LinkButton
                href={withNext({ route: AppRoute.SetPassword, next: AppRoute.Account })}
                variant={ButtonVariant.Secondary}
              >
                {account.hasPassword ? t("account.security.change") : t("account.security.set")}
              </LinkButton>
            </li>
            <li className={styles.row}>
              <Icon name="passkey" size={20} className={styles.rowIcon} />
              <span className={styles.rowText}>
                <strong>{t("account.security.passkeys")}</strong>
                <span>{tCount({ base: "account.security.passkeyCount", count: account.passkeyCount })}</span>
              </span>
              <LinkButton
                href={withNext({ route: AppRoute.AddPasskey, next: AppRoute.Account })}
                variant={ButtonVariant.Secondary}
              >
                {t("account.security.addPasskey")}
              </LinkButton>
            </li>
          </ul>
        </section>
      </div>

      <section className={styles.leave} aria-labelledby="leave-title">
        <div>
          <h2 id="leave-title">{t("account.signOut.title")}</h2>
          <p>{t("account.signOut.lead")}</p>
        </div>
        <Button variant={ButtonVariant.Secondary} onClick={leave} busy={leaving}>
          <Icon name="logOut" size={16} />
          {t("app.menu.signOut")}
        </Button>
      </section>
    </div>
  );
}
