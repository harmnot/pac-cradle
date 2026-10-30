"use client";

import { useParams } from "next/navigation";
import { useMemo } from "react";
import { Button, ButtonSize, ButtonVariant, LinkButton } from "@/components/ui/Button";
import { Icon } from "@/components/ui/Icon";
import { Notice, NoticeTone } from "@/components/ui/Notice";
import { Skeleton, SkeletonShape } from "@/components/ui/Skeleton";
import { formatMm } from "@/features/capture/format";
import { AppRoute } from "@/features/identity/routes";
import { describeError } from "@/lib/errors";
import { t, tCount } from "@/lib/i18n";
import { none, some } from "@/lib/maybe";
import { useApiQuery } from "@/lib/use-api";
import { getDesign, type SavedDesign } from "../api";
import { runPreview } from "../preview";
import type { Design } from "../types";
import { useInsertCore } from "../use-insert-core";
import { PackagingPreview } from "./PackagingPreview";
import styles from "./SavedDesignView.module.css";

/** Codes that mean this design can't be read from here. */
const NOT_HERE = new Set(["DESIGN_NOT_FOUND", "AUTH_SESSION_EXPIRED", "INVALID_REQUEST"]);

/** What the design is, as tags: style, how many, board, box. */
function Facts({ spec }: { readonly spec: Design }) {
  const count = spec.products.reduce((sum, product) => sum + product.quantity, 0);
  const facts = [
    { icon: "insert" as const, text: t(`style.${spec.insert.style}`) },
    { icon: "bottle" as const, text: tCount({ base: "design.products", count }) },
    {
      icon: "layers" as const,
      text: t("saved.board", { board: t(`board.${spec.material.board}`), mm: formatMm(spec.material.thickness_mm) }),
    },
    {
      icon: "box" as const,
      text:
        spec.outer_box === null
          ? t("choices.box.none")
          : t("saved.box", { box: t(`choices.box.${spec.outer_box.style}`), board: t(`board.${spec.outer_box.material.board}`) }),
    },
  ];
  return (
    <ul className={styles.facts}>
      {facts.map((fact, index) => (
        <li key={fact.text} style={{ animationDelay: `${200 + index * 70}ms` }}>
          <Icon name={fact.icon} size={18} />
          {fact.text}
        </li>
      ))}
    </ul>
  );
}

/** A loaded design: its name, what it is, and its preview. */
function Loaded({ design }: { readonly design: SavedDesign }) {
  const { engine, retry } = useInsertCore();
  const frame = useMemo(
    () => (engine.kind === "ready" ? some(runPreview({ core: engine.core, design: design.spec })) : none),
    [engine, design.spec],
  );
  return (
    <div className={styles.layout}>
      <div className={styles.side}>
        <header className={styles.head}>
          <p className={styles.eyebrow}>
            <span className={styles.stamp} aria-hidden="true">
              <Icon name="check" size={14} />
            </span>
            {t("saved.eyebrow", { version: design.version })}
          </p>
          <h1 className={styles.title}>{design.name === "" ? t("design.untitled") : design.name}</h1>
        </header>
        <Facts spec={design.spec} />
        <Notice>{t("saved.next")}</Notice>
        <div className={styles.actions}>
          <LinkButton href={AppRoute.Capture} variant={ButtonVariant.Secondary}>
            {t("saved.another")}
          </LinkButton>
          <LinkButton href={AppRoute.Workspace} variant={ButtonVariant.Quiet}>
            {t("capture.toWorkspace")}
          </LinkButton>
        </div>
      </div>
      <PackagingPreview
        engine={engine}
        onRetry={retry}
        design={some(design.spec)}
        frame={frame}
        waiting={t("preview.loading")}
      />
    </div>
  );
}

/**
 * A saved design (`/design/{id}`): what it is and its preview. The 2D editor (M1-E5) and the
 * 3D fold (E6) open here next.
 */
export function SavedDesignView() {
  const { designId } = useParams<{ designId: string }>();
  const query = useApiQuery({
    key: `/designs/${designId}`,
    load: (signal) => getDesign({ id: designId, signal }),
  });

  if (query.value.kind === "some") {
    return (
      <div className={styles.page}>
        <LinkButton href={AppRoute.Workspace} variant={ButtonVariant.Quiet}>
          {t("capture.toWorkspace")}
        </LinkButton>
        <Loaded design={query.value.value} />
      </div>
    );
  }
  if (query.error.kind === "some" && NOT_HERE.has(query.error.value.code)) {
    return (
      <div className={styles.missing}>
        <h1 className={styles.title}>{t("saved.missing.title")}</h1>
        <p className={styles.lead}>{t("saved.missing.lead")}</p>
        <LinkButton href={AppRoute.Workspace} size={ButtonSize.Large} withArrow>
          {t("capture.toWorkspace")}
        </LinkButton>
      </div>
    );
  }
  if (query.error.kind === "some") {
    return (
      <div className={styles.missing}>
        <Notice
          tone={NoticeTone.Error}
          action={
            <Button variant={ButtonVariant.Secondary} onClick={query.reload}>
              {t("session.retry")}
            </Button>
          }
        >
          {describeError(query.error.value)}
        </Notice>
      </div>
    );
  }
  return (
    <div className={styles.page} aria-busy="true">
      <div className={styles.layout}>
        <Skeleton shape={SkeletonShape.Title} count={2} />
        <Skeleton shape={SkeletonShape.Block} />
      </div>
    </div>
  );
}
