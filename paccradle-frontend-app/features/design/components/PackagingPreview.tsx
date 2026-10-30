"use client";

import { startTransition, useId, useState, ViewTransition, type CSSProperties, type KeyboardEvent } from "react";
import { Button, ButtonVariant } from "@/components/ui/Button";
import { Icon } from "@/components/ui/Icon";
import { Notice, NoticeTone } from "@/components/ui/Notice";
import { formatMm } from "@/features/capture/format";
import { t, tCount, type MessageKey } from "@/lib/i18n";
import type { Maybe } from "@/lib/maybe";
import { emptySpaceShare } from "../choices";
import {
  expand,
  fitSummary,
  layOutPieces,
  planView,
  polygonPoints,
  ringsData,
  type PreviewFrame,
  type Rect,
} from "../preview";
import { pathData } from "../svg";
import type { BoxSize, Design, Packaging, ProductFit } from "../types";
import type { EngineState } from "../use-insert-core";
import styles from "./PackagingPreview.module.css";

/**
 * The live preview beside the design choices (M1-E4) and on a saved design: the insert in
 * its box seen from above, or every piece flat (cut solid, crease dashed), with the sizes,
 * the empty space and how the products fit. Choices made in a transition morph the drawing.
 */

/** The two drawings. */
const PreviewView = {
  Plan: "plan",
  Flat: "flat",
} as const;

type PreviewView = (typeof PreviewView)[keyof typeof PreviewView];

const VIEWS: ReadonlyArray<{ readonly view: PreviewView; readonly label: MessageKey }> = [
  { view: PreviewView.Plan, label: "preview.view.plan" },
  { view: PreviewView.Flat, label: "preview.view.flat" },
];

/** Room round a drawing, as a share of its larger side; more on the plan for the dimensions. */
const PLAN_MARGIN_SHARE = 0.14;
const FLAT_MARGIN_SHARE = 0.04;
/** Gap between flat pieces, in mm. */
const FLAT_GAP_MM = 12;
/** Dimension labels' size, as a share of the drawing's larger side. */
const LABEL_SHARE = 0.042;
/** The EU's coming cap on empty space in e-commerce packaging (PPWR, from 2030; CLAUDE.md U11). */
const EU_EMPTY_SPACE_CAP = 0.5;
const PERCENT = 100;

/** Props for the preview panel. */
export interface PackagingPreviewProps {
  readonly engine: EngineState;
  readonly onRetry: () => void;
  /** The design drawn, once there is one (the height is known). */
  readonly design: Maybe<Design>;
  readonly frame: Maybe<PreviewFrame>;
  /** Shown while there's no design yet. */
  readonly waiting: string;
}

/** "182 × 128 × 92 mm". */
function size3d(size: BoxSize): string {
  return t("preview.size3d", {
    l: formatMm(size.length_mm),
    w: formatMm(size.width_mm),
    h: formatMm(size.height_mm),
  });
}

/** The viewBox for a drawing of `extent`, with a margin. */
function viewBoxOf({ extent, share }: { extent: Rect; share: number }): { readonly box: Rect; readonly unit: number } {
  const unit = Math.max(extent.width, extent.height, 1);
  return { box: expand({ rect: extent, by: unit * share }), unit };
}

/** The insert in its box, from above: walls, board, products in their fit colours, dimensions. */
function PlanDrawing({ design, packaging, fits }: { readonly design: Design; readonly packaging: Packaging; readonly fits: readonly ProductFit[] }) {
  const plan = planView({ design, packaging, fits });
  const { box, unit } = viewBoxOf({ extent: plan.extent, share: PLAN_MARGIN_SHARE });
  const label = unit * LABEL_SHARE;
  const frame = plan.box === null ? plan.insert : plan.box.inside;
  const outer = plan.box === null ? plan.insert : plan.box.walls;
  const dimension = {
    length: formatMm(frame.width),
    width: formatMm(frame.height),
    topY: outer.y - label * 1.3,
    leftX: outer.x - label * 1.3,
  };
  return (
    <svg
      className={styles.drawing}
      viewBox={`${box.x} ${box.y} ${box.width} ${box.height}`}
      role="img"
      aria-label={t("preview.plan.label", { l: dimension.length, w: dimension.width })}
      style={{ "--label": `${label}px` } as CSSProperties}
    >
      {plan.box !== null && (
        <>
          <rect className={styles.floor} x={plan.box.inside.x} y={plan.box.inside.y} width={plan.box.inside.width} height={plan.box.inside.height} />
          <path
            className={styles.walls}
            d={ringsData({
              outline: rectRing(plan.box.walls),
              holes: [rectRing(plan.box.inside)],
            })}
          />
        </>
      )}
      <g className={styles.board}>
        {plan.board.map((shape, index) =>
          shape.kind === "panel" ? (
            <path key={index} d={ringsData({ outline: shape.outline, holes: shape.holes })} />
          ) : (
            <rect key={index} x={shape.rect.x} y={shape.rect.y} width={shape.rect.width} height={shape.rect.height} />
          ),
        )}
      </g>
      <g className={styles.products}>
        {plan.products.map((product, index) => (
          <polygon
            key={index}
            className={styles.product}
            data-fit={product.color}
            points={polygonPoints(product.outline)}
            style={{ animationDelay: `${Math.min(index, 24) * 30}ms` }}
          />
        ))}
      </g>
      <g className={styles.dimensions} aria-hidden="true">
        <path d={`M${frame.x} ${dimension.topY}H${frame.x + frame.width}`} />
        <path d={`M${frame.x} ${dimension.topY - label / 2}v${label}M${frame.x + frame.width} ${dimension.topY - label / 2}v${label}`} />
        <text x={frame.x + frame.width / 2} y={dimension.topY - label * 0.45} textAnchor="middle">
          {t("preview.mm", { mm: dimension.length })}
        </text>
        <path d={`M${dimension.leftX} ${frame.y}V${frame.y + frame.height}`} />
        <path d={`M${dimension.leftX - label / 2} ${frame.y}h${label}M${dimension.leftX - label / 2} ${frame.y + frame.height}h${label}`} />
        <text
          x={dimension.leftX - label * 0.45}
          y={frame.y + frame.height / 2}
          textAnchor="middle"
          transform={`rotate(-90 ${dimension.leftX - label * 0.45} ${frame.y + frame.height / 2})`}
        >
          {t("preview.mm", { mm: dimension.width })}
        </text>
      </g>
    </svg>
  );
}

function rectRing(rect: Rect): ReadonlyArray<readonly [number, number]> {
  return [
    [rect.x, rect.y],
    [rect.x + rect.width, rect.y],
    [rect.x + rect.width, rect.y + rect.height],
    [rect.x, rect.y + rect.height],
  ];
}

/** Every piece flat, as it's cut: the insert's on cream, the box's on kraft. */
function FlatDrawing({ packaging }: { readonly packaging: Packaging }) {
  const laid = layOutPieces({ packaging, gapMm: FLAT_GAP_MM });
  const { box, unit } = viewBoxOf({ extent: laid.extent, share: FLAT_MARGIN_SHARE });
  const label = unit * LABEL_SHARE;
  return (
    <svg
      className={styles.drawing}
      viewBox={`${box.x} ${box.y} ${box.width} ${box.height}`}
      role="img"
      aria-label={t("preview.flat.label")}
      style={{ "--label": `${label}px` } as CSSProperties}
    >
      {laid.pieces.map(({ piece, part, shift, frame }, index) => (
        <g key={index} className={styles.piece} data-part={part} style={{ animationDelay: `${index * 70}ms` }}>
          <g transform={`translate(${shift[0]} ${shift[1]})`}>
            <path className={styles.sheet} d={[piece.outline, ...piece.holes].map(pathData).join(" ")} />
            {piece.creases.map((crease, creaseIndex) => (
              <path key={`c${creaseIndex}`} className={styles.crease} d={pathData(crease)} />
            ))}
            {piece.perforations.map((perforation, perforationIndex) => (
              <path key={`p${perforationIndex}`} className={styles.perforation} d={pathData(perforation)} />
            ))}
            {[piece.outline, ...piece.holes].map((cut, cutIndex) => (
              <path key={`k${cutIndex}`} className={styles.cut} d={pathData(cut)} />
            ))}
          </g>
          {piece.quantity > 1 && (
            <text className={styles.quantity} x={frame.x + frame.width} y={frame.y - label * 0.3} textAnchor="end">
              {t("preview.quantity", { count: piece.quantity })}
            </text>
          )}
        </g>
      ))}
    </svg>
  );
}

/** The cut, crease and fit keys under the drawing. */
function Legend({ view }: { readonly view: PreviewView }) {
  return (
    <ul className={styles.legend} aria-label={t("preview.legend")}>
      {view === PreviewView.Flat ? (
        <>
          <li>
            <span className={styles.keyCut} aria-hidden="true" />
            {t("preview.key.cut")}
          </li>
          <li>
            <span className={styles.keyCrease} aria-hidden="true" />
            {t("preview.key.crease")}
          </li>
        </>
      ) : (
        <>
          <li>
            <span className={styles.keyFit} data-fit="green" aria-hidden="true" />
            {t("preview.key.fits")}
          </li>
          <li>
            <span className={styles.keyFit} data-fit="yellow" aria-hidden="true" />
            {t("preview.key.tight")}
          </li>
          <li>
            <span className={styles.keyFit} data-fit="red" aria-hidden="true" />
            {t("preview.key.collides")}
          </li>
        </>
      )}
    </ul>
  );
}

/** How the products fit, in one line. */
function FitLine({ fits }: { readonly fits: readonly ProductFit[] }) {
  if (fits.length === 0) {
    return null;
  }
  const summary = fitSummary(fits);
  const tone = summary.red > 0 ? "red" : summary.yellow > 0 ? "yellow" : "green";
  const text =
    tone === "green"
      ? tCount({ base: "preview.fit.all", count: fits.length, vars: { mm: formatMm(summary.tightestMm) } })
      : t("preview.fit.some", { tight: summary.yellow, collide: summary.red });
  return (
    <p className={styles.fit} data-fit={tone}>
      <span className={styles.fitDot} aria-hidden="true" />
      {text}
    </p>
  );
}

/** How many things the checks found to fix before cutting, or that there are none. */
function ChecksLine({ count }: { readonly count: number }) {
  return (
    <p className={styles.checks} data-clear={count === 0}>
      <Icon name={count === 0 ? "check" : "alert"} size={16} />
      {count === 0 ? t("preview.checks.none") : tCount({ base: "preview.checks", count })}
    </p>
  );
}

/** The sizes, the pieces to cut, and the empty space against the EU's coming cap. */
function Stats({ design, packaging }: { readonly design: Design; readonly packaging: Packaging }) {
  const { insert } = packaging;
  const box = packaging.outer_box;
  const empty = emptySpaceShare({ design, packaging });
  const pieces =
    insert.pieces.reduce((sum, piece) => sum + piece.quantity, 0) +
    (box === null ? 0 : box.pieces.reduce((sum, piece) => sum + piece.quantity, 0));
  return (
    <dl className={styles.stats}>
      <div>
        <dt>{t("preview.stat.insert")}</dt>
        <dd>{size3d({ length_mm: insert.length_mm, width_mm: insert.width_mm, height_mm: insert.height_mm })}</dd>
      </div>
      {box !== null && (
        <>
          <div>
            <dt>{t("preview.stat.inside")}</dt>
            <dd>{size3d(box.inside_mm)}</dd>
          </div>
          <div>
            <dt>{t("preview.stat.outside")}</dt>
            <dd>{size3d(box.outside_mm)}</dd>
          </div>
        </>
      )}
      <div>
        <dt>{t("preview.stat.pieces")}</dt>
        <dd>{tCount({ base: "preview.pieces", count: pieces })}</dd>
      </div>
      {empty.kind === "some" && (
        <div className={styles.empty} style={{ "--share": empty.value, "--cap": EU_EMPTY_SPACE_CAP } as CSSProperties}>
          <dt>{t("preview.stat.empty")}</dt>
          <dd>
            <strong>{t("preview.percent", { value: Math.round(empty.value * PERCENT) })}</strong>
            <span className={styles.meter} aria-hidden="true">
              <span className={styles.meterFill} data-over={empty.value > EU_EMPTY_SPACE_CAP} />
              <span className={styles.meterCap} />
            </span>
            <span className={styles.emptyNote}>{t("preview.empty.note")}</span>
          </dd>
        </div>
      )}
    </dl>
  );
}

/** The drawing's frame, for what isn't a drawing yet (loading, waiting, a problem). */
function Placeholder({ busy, children }: { readonly busy: boolean; readonly children: string }) {
  return (
    <div className={styles.placeholder} data-busy={busy}>
      <span className={styles.placeholderBox} aria-hidden="true" />
      <p role="status">{children}</p>
    </div>
  );
}

/** The live preview panel. */
export function PackagingPreview({ engine, onRetry, design, frame, waiting }: PackagingPreviewProps) {
  const [view, setView] = useState<PreviewView>(PreviewView.Plan);
  const id = useId();

  /** Switches the drawing; in a transition, so it morphs. */
  const show = (next: PreviewView) => startTransition(() => setView(next));

  const keyTabs = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") {
      return;
    }
    event.preventDefault();
    const next = view === PreviewView.Plan ? PreviewView.Flat : PreviewView.Plan;
    show(next);
    document.getElementById(`${id}-${next}`)?.focus();
  };

  const body = () => {
    if (engine.kind === "loading") {
      return <Placeholder busy>{t("preview.loading")}</Placeholder>;
    }
    if (engine.kind === "failed") {
      return (
        <Notice
          tone={NoticeTone.Error}
          action={
            <Button variant={ButtonVariant.Secondary} onClick={onRetry}>
              {t("session.retry")}
            </Button>
          }
        >
          {t("preview.engineFailed")}
        </Notice>
      );
    }
    if (design.kind === "none" || frame.kind === "none") {
      return <Placeholder busy={false}>{waiting}</Placeholder>;
    }
    if (frame.value.kind === "error") {
      return (
        <div className={styles.problem}>
          <Notice tone={NoticeTone.Error}>{t("preview.cant")}</Notice>
          <p className={styles.detail}>{frame.value.message}</p>
        </div>
      );
    }
    const { packaging, fits, warnings } = frame.value;
    return (
      <>
        <ViewTransition update="morph" default="none">
          <figure className={styles.figure} id={`${id}-panel`} role="tabpanel" aria-labelledby={`${id}-${view}`}>
            {view === PreviewView.Plan ? (
              <PlanDrawing design={design.value} packaging={packaging} fits={fits} />
            ) : (
              <FlatDrawing packaging={packaging} />
            )}
          </figure>
        </ViewTransition>
        <Legend view={view} />
        {view === PreviewView.Plan && <FitLine fits={fits} />}
        <ChecksLine count={warnings.length} />
        <Stats design={design.value} packaging={packaging} />
      </>
    );
  };

  return (
    <section className={styles.panel} aria-labelledby={`${id}-title`}>
      <header className={styles.head}>
        <h2 id={`${id}-title`} className={styles.title}>
          {t("preview.title")}
        </h2>
        <div role="tablist" aria-label={t("preview.views")} className={styles.tabs} onKeyDown={keyTabs}>
          {VIEWS.map((option) => (
            <button
              key={option.view}
              id={`${id}-${option.view}`}
              type="button"
              role="tab"
              aria-selected={view === option.view}
              aria-controls={`${id}-panel`}
              tabIndex={view === option.view ? 0 : -1}
              className={styles.tab}
              onClick={() => show(option.view)}
            >
              {t(option.label)}
            </button>
          ))}
        </div>
      </header>
      {body()}
    </section>
  );
}
