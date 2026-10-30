"use client";

import Link from "next/link";
import { TransitionType } from "@/components/motion/PageTransition";
import { revealDelay } from "@/components/motion/reveal";
import { CountUp } from "@/components/motion/CountUp";
import { Button, ButtonSize, ButtonVariant, LinkButton } from "@/components/ui/Button";
import { Icon, type IconName } from "@/components/ui/Icon";
import { Notice, NoticeTone } from "@/components/ui/Notice";
import { Skeleton, SkeletonShape } from "@/components/ui/Skeleton";
import {
  designQueryString,
  InsertStyle,
  listDesigns,
  type DesignQuery,
  type DesignSummary,
} from "@/features/design/api";
import { designPath } from "@/features/design/routes";
import { fetchMe } from "@/features/identity/api";
import { SessionUnreachable } from "@/features/identity/components/RequireAccount";
import { AppRoute, withNext } from "@/features/identity/routes";
import { isAccount, useSession, type SessionState } from "@/features/identity/session";
import { describeError } from "@/lib/errors";
import { formatRelative, t, tCount, type MessageKey } from "@/lib/i18n";
import { useApiQuery } from "@/lib/use-api";
import styles from "./WorkspaceHome.module.css";

/** The newest designs the workspace shows. */
const RECENT: DesignQuery = { page: 1, pageSize: 6, sort: "updated_at", order: "desc" };
const MORNING_ENDS = 12;
const AFTERNOON_ENDS = 18;

const STYLE_ICON: Readonly<Record<InsertStyle, IconName>> = {
  [InsertStyle.PartitionGrid]: "grid",
  [InsertStyle.CutoutCard]: "insert",
  [InsertStyle.CradlePlatform]: "bottle",
};

const STYLE_LABEL: Readonly<Record<InsertStyle, MessageKey>> = {
  [InsertStyle.PartitionGrid]: "style.partition_grid",
  [InsertStyle.CutoutCard]: "style.cutout_card",
  [InsertStyle.CradlePlatform]: "style.cradle_platform",
};

function greeting(): string {
  const hour = new Date().getHours();
  if (hour < MORNING_ENDS) {
    return t("workspace.greeting.morning");
  }
  return hour < AFTERNOON_ENDS ? t("workspace.greeting.afternoon") : t("workspace.greeting.evening");
}

/** The badge a design wears once paid for. */
function UnlockBadge({ design }: { readonly design: DesignSummary }) {
  if (design.unlock.kind === "none") {
    return null;
  }
  const covered = design.unlock.value.coversCurrentVersion;
  return (
    <span className={`${styles.badge} ${covered ? styles.paid : styles.changed}`}>
      {covered ? t("design.badge.paid") : t("design.badge.changed")}
    </span>
  );
}

/** One recent design. */
function DesignCard({
  design,
  index,
  now,
}: {
  readonly design: DesignSummary;
  readonly index: number;
  readonly now: number;
}) {
  return (
    <li className={styles.design} style={revealDelay(index, 70)}>
      <span className={styles.designIcon} aria-hidden="true">
        <Icon name={STYLE_ICON[design.style]} size={22} />
      </span>
      <span className={styles.designText}>
        <Link href={designPath(design.id)} className={styles.designLink} transitionTypes={[TransitionType.Forward]}>
          <strong>{design.name === "" ? t("design.untitled") : design.name}</strong>
        </Link>
        <span>
          {t(STYLE_LABEL[design.style])} · {tCount({ base: "design.products", count: design.productCount })}
        </span>
        <span className={styles.updated}>
          {t("design.updated", { when: formatRelative({ timestamp: design.updatedAt, now }) })}
        </span>
      </span>
      <UnlockBadge design={design} />
    </li>
  );
}

/** The newest designs, with loading, empty and error states. */
function RecentDesigns({ session }: { readonly session: SessionState["status"] }) {
  const enabled = session === "active";
  const designs = useApiQuery({
    key: `/designs?${designQueryString(RECENT)}`,
    load: async (signal) => {
      const result = await listDesigns({ query: RECENT, signal });
      // "Updated 3 days ago" counts from when the list arrived, never from a render.
      return result.kind === "ok" ? { ...result, value: { page: result.value, readAt: Date.now() } } : result;
    },
    enabled,
  });

  if (designs.status === "error" && designs.value.kind === "none") {
    return (
      <Notice
        tone={NoticeTone.Error}
        action={
          <Button variant={ButtonVariant.Secondary} onClick={designs.reload}>
            {t("session.retry")}
          </Button>
        }
      >
        {designs.error.kind === "some" ? describeError(designs.error.value) : t("error.generic")}
      </Notice>
    );
  }
  if (session === "none") {
    return <EmptyDesigns />;
  }
  if (designs.value.kind === "none") {
    return <Skeleton shape={SkeletonShape.Block} count={2} />;
  }
  const { page, readAt } = designs.value.value;
  if (page.items.length === 0) {
    return <EmptyDesigns />;
  }
  return (
    <>
      <ul className={styles.designs}>
        {page.items.map((design, index) => (
          <DesignCard key={design.id} design={design} index={index} now={readAt} />
        ))}
      </ul>
      {page.total > page.items.length && (
        <p className={styles.more}>
          {tCount({ base: "workspace.designs.more", count: page.total - page.items.length })}
        </p>
      )}
    </>
  );
}

function EmptyDesigns() {
  return (
    <div className={styles.empty}>
      <svg viewBox="0 0 120 80" aria-hidden="true" className={styles.emptyArt}>
        <rect x="14" y="14" width="92" height="56" rx="8" pathLength={1} />
        <path d="M14 30h92" pathLength={1} />
        <circle cx="44" cy="50" r="10" pathLength={1} />
        <circle cx="76" cy="50" r="10" pathLength={1} />
      </svg>
      <p>{t("workspace.designs.empty")}</p>
    </div>
  );
}

/** Plan, credits and unlocked designs, counting up as they arrive. */
function PlanStrip() {
  const me = useApiQuery({ key: "/me", load: (signal) => fetchMe(signal) });
  if (me.value.kind === "none") {
    return <Skeleton shape={SkeletonShape.Block} />;
  }
  const { billing } = me.value.value;
  const stats: ReadonlyArray<{ readonly label: string; readonly value: number | string; readonly icon: IconName }> = [
    { label: t("workspace.stat.plan"), value: t(`plan.${billing.plan}`), icon: "leaf" },
    { label: t("workspace.stat.credits"), value: billing.credits, icon: "credit" },
    { label: t("workspace.stat.unlocked"), value: billing.unlockedDesigns, icon: "check" },
  ];
  return (
    <ul className={styles.stats}>
      {stats.map((stat, index) => (
        <li key={stat.label} className={styles.stat} style={revealDelay(index, 90)}>
          <Icon name={stat.icon} size={20} className={styles.statIcon} />
          <span className={styles.statValue}>
            {typeof stat.value === "number" ? <CountUp value={stat.value} durationMs={900} /> : stat.value}
          </span>
          <span className={styles.statLabel}>{stat.label}</span>
        </li>
      ))}
    </ul>
  );
}

/**
 * The workspace: a greeting, what the account has, the way into a new insert, and the newest
 * designs. Visitors and guests see the same room with an invitation to sign up.
 */
export function WorkspaceHome() {
  const { state } = useSession();
  const account = isAccount(state);
  const guest = state.status === "active" && !account;

  if (state.status === "unreachable") {
    return <SessionUnreachable />;
  }

  return (
    <div className={styles.home}>
      <section className={styles.hello} aria-labelledby="workspace-title">
        <p className={styles.eyebrow}>{t("workspace.eyebrow")}</p>
        <h1 id="workspace-title" className={styles.title}>
          {account ? greeting() : t("workspace.visitor.title")}
        </h1>
        <p className={styles.lead}>
          {isAccount(state) && state.account.email.kind === "some"
            ? t("workspace.signedInAs", { email: state.account.email.value })
            : t("workspace.visitor.lead")}
        </p>
        {!account && state.status !== "starting" && (
          <div className={styles.actions}>
            <LinkButton href={withNext({ route: AppRoute.SignUp, next: AppRoute.Workspace })} withArrow>
              {guest ? t("workspace.guest.save") : t("app.signUp")}
            </LinkButton>
            <LinkButton href={AppRoute.SignIn} variant={ButtonVariant.Secondary}>
              {t("app.signIn")}
            </LinkButton>
          </div>
        )}
      </section>

      {account && <PlanStrip />}

      <section className={styles.start} aria-labelledby="start-title">
        <div className={styles.startText}>
          <p className={styles.startBadge}>{t("workspace.start.badge")}</p>
          <h2 id="start-title">{t("workspace.start.title")}</h2>
          <p>{t("workspace.start.lead")}</p>
          <div className={styles.startAction}>
            <LinkButton href={AppRoute.Capture} variant={ButtonVariant.Accent} size={ButtonSize.Large} withArrow>
              {t("workspace.start.action")}
            </LinkButton>
          </div>
        </div>
        <ol className={styles.startSteps}>
          <li>
            <Icon name="camera" size={20} />
            {t("workspace.start.step.photo")}
          </li>
          <li>
            <Icon name="insert" size={20} />
            {t("workspace.start.step.insert")}
          </li>
          <li>
            <Icon name="file" size={20} />
            {t("workspace.start.step.files")}
          </li>
        </ol>
      </section>

      <section className={styles.recent} aria-labelledby="recent-title">
        <h2 id="recent-title" className={styles.sectionTitle}>
          {t("workspace.designs.title")}
        </h2>
        <RecentDesigns session={state.status} />
      </section>
    </div>
  );
}
