"use client";

import { useParams } from "next/navigation";
import { useEffect, useState, type CSSProperties } from "react";
import { CountUp } from "@/components/motion/CountUp";
import { TransitionType } from "@/components/motion/PageTransition";
import { Button, ButtonSize, ButtonVariant, LinkButton } from "@/components/ui/Button";
import { Icon } from "@/components/ui/Icon";
import { Notice, NoticeTone } from "@/components/ui/Notice";
import { Skeleton, SkeletonShape } from "@/components/ui/Skeleton";
import { newDesignFromCapture } from "@/features/design/routes";
import { describeError } from "@/lib/errors";
import { t, tCount, type MessageKey } from "@/lib/i18n";
import { none, some } from "@/lib/maybe";
import { formatMm, sizeLabel } from "../format";
import { shownPhoto } from "../photo-memory";
import {
  CaptureMode,
  CaptureStatus,
  type Capture,
  type PhotoWarning,
  type WireMeasuredProduct,
} from "../types";
import { useCapture } from "../use-capture";
import { FigureTool, OutlineEntrance, OutlineFigure } from "./OutlineFigure";
import { OutlineReview, type Corrected } from "./OutlineReview";
import { PinState, TapPin } from "./TapLayer";
import styles from "./CaptureView.module.css";

/** Where a new capture starts, in a given mode (`?mode=`, read by CaptureFlow). */
function captureLink(mode: "photo" | "trace" | "typed" | "any"): string {
  return mode === "any" ? "/capture" : `/capture?mode=${mode}`;
}

/** The measuring screen's lines, one after another while vision works. */
const MEASURING_LINES: readonly MessageKey[] = [
  "capture.measuring.paper",
  "capture.measuring.flatten",
  "capture.measuring.outline",
  "capture.measuring.mm",
];
const MEASURING_LINE_MS = 1400;

/** What failed, and the ways on offer, by the run's error code. */
interface FailureHelp {
  readonly title: MessageKey;
  /** Tapping the product on the photo may find it (vision runs again at the tap). */
  readonly tap: boolean;
  readonly retake: boolean;
  readonly trace: boolean;
  readonly typed: boolean;
}

const FAILURES: Readonly<Record<string, FailureHelp>> = {
  CAPTURE_PAPER_NOT_FOUND: { title: "capture.fail.paper", tap: false, retake: true, trace: false, typed: true },
  CAPTURE_PRODUCT_NOT_FOUND: { title: "capture.fail.product", tap: true, retake: true, trace: true, typed: true },
  CAPTURE_NOTHING_AT_TAP: { title: "capture.fail.nothingAtTap", tap: true, retake: true, trace: true, typed: true },
  CAPTURE_TRACE_NOT_FOUND: { title: "capture.fail.trace", tap: false, retake: true, trace: false, typed: true },
  CAPTURE_PHOTO_DELETED: { title: "capture.fail.deleted", tap: false, retake: true, trace: false, typed: false },
};
const GENERIC_FAILURE: FailureHelp = { title: "capture.fail.generic", tap: false, retake: true, trace: true, typed: true };

/** Codes that mean this capture can't be read from here. */
const NOT_HERE = new Set(["CAPTURE_NOT_FOUND", "AUTH_SESSION_EXPIRED", "INVALID_REQUEST"]);

/** The photo to show for a capture (this tab's upload, else its signed link). */
function photoOf(capture: Capture) {
  return shownPhoto({ captureId: capture.id, link: capture.photo.kind === "some" ? some(capture.photo.value.url) : none });
}

/** The stored error's words for the person. */
function failureText(capture: Capture): string {
  return capture.error.kind === "some" ? describeError({ status: 0, ...capture.error.value }) : t("error.generic");
}

function warningText(warning: PhotoWarning): string {
  switch (warning.kind) {
    case "outline_may_be_bigger":
      return t("warning.outline_may_be_bigger", {
        up_to_mm: formatMm(warning.up_to_mm),
        height_mm: formatMm(warning.height_mm),
      });
    case "other_objects":
      return tCount({ base: "warning.other_objects", count: warning.count });
    default:
      return t(`warning.${warning.kind}`);
  }
}

function isParallax(warning: PhotoWarning): boolean {
  return warning.kind === "tall_product_looks_bigger" || warning.kind === "outline_may_be_bigger";
}

/** One product's size, shape and how sure vision is. */
function ProductFacts({ product, index, count }: { readonly product: WireMeasuredProduct; readonly index: number; readonly count: number }) {
  const percent = Math.round(product.confidence * 100);
  const round = product.shape === "round" && product.diameter_mm !== null;
  const size = round
    ? sizeLabel({ shape: "round", diameterMm: product.diameter_mm ?? 0 })
    : sizeLabel({ shape: "box", lengthMm: product.bbox_mm.l, widthMm: product.bbox_mm.w });
  const userOutline = product.contour_source === "user";
  return (
    <section className={styles.product} aria-label={t("capture.result.product", { n: index + 1 })}>
      {count > 1 && <p className={styles.productName}>{t("capture.result.product", { n: index + 1 })}</p>}
      <p className={styles.size}>{size}</p>
      <dl className={styles.facts}>
        <div>
          <dt>{t("capture.result.shape")}</dt>
          <dd>{t(`capture.shape.${product.shape}`)}</dd>
        </div>
        <div>
          <dt>{t("capture.result.area")}</dt>
          <dd>{t("capture.size.area", { area: formatMm(product.area_mm2 / 100) })}</dd>
        </div>
        <div>
          <dt>{t("capture.result.height")}</dt>
          <dd>
            {product.height_mm === null
              ? t("capture.result.heightLater")
              : t("capture.size.mm", { mm: formatMm(product.height_mm) })}
          </dd>
        </div>
      </dl>
      <div className={styles.confidence} style={{ "--confidence": percent } as CSSProperties}>
        <svg viewBox="0 0 40 40" aria-hidden="true">
          <circle className={styles.ringTrack} cx="20" cy="20" r="16" />
          <circle className={styles.ring} cx="20" cy="20" r="16" pathLength={100} />
        </svg>
        <span>
          {userOutline ? (
            t("capture.result.yourOutline")
          ) : (
            <>
              <strong>
                <CountUp value={percent} durationMs={900} />%
              </strong>{" "}
              {t("capture.result.confidence")}
            </>
          )}
        </span>
      </div>
    </section>
  );
}

/** Props for the measured screen. */
interface MeasuredProps {
  readonly capture: Capture;
  readonly onCorrected: (capture: Capture) => void;
}

/**
 * Measured: the outline on the photo with the ways to fix it (tap the product, adjust the
 * points), the sizes, any warnings, and what comes next. Also shown after a later run failed
 * (a tap that found nothing), with the reason, as the last outline is kept.
 */
function Measured({ capture, onCorrected }: MeasuredProps) {
  const [tool, setTool] = useState<FigureTool>(FigureTool.None);
  const [entrance, setEntrance] = useState<OutlineEntrance>(OutlineEntrance.Draw);
  const [saved, setSaved] = useState(false);
  if (capture.result.kind === "none") {
    return null;
  }
  const result = capture.result.value;
  const parallax = result.warnings.some(isParallax);
  const failed = capture.status === CaptureStatus.Failed;

  const corrected = ({ capture: next, kind }: Corrected) => {
    setTool(FigureTool.None);
    if (kind === "edit") {
      setEntrance(OutlineEntrance.Settle);
      setSaved(true);
    }
    onCorrected(next);
  };

  const changeTool = (next: FigureTool) => {
    setSaved(false);
    setTool(next);
  };

  return (
    <div className={styles.layout}>
      <OutlineReview
        capture={capture}
        tool={tool}
        onTool={changeTool}
        offerTools
        entrance={entrance}
        onCorrected={corrected}
      />
      <div className={styles.side}>
        <header className={styles.head}>
          <p className={styles.eyebrow}>{t("capture.result.eyebrow")}</p>
          <h1 className={styles.title}>{t("capture.result.title")}</h1>
          <p className={styles.lead}>
            {t(capture.paper === "letter" ? "capture.result.onLetter" : "capture.result.onA4")}
          </p>
        </header>
        {failed && (
          <Notice tone={NoticeTone.Error}>
            {failureText(capture)} {t("capture.result.lastKept")}
          </Notice>
        )}
        {saved && <Notice tone={NoticeTone.Success}>{t("capture.adjust.saved")}</Notice>}
        {result.products.map((product, index) => (
          <ProductFacts key={product.id} product={product} index={index} count={result.products.length} />
        ))}
        {result.warnings.length > 0 && (
          <ul className={styles.warnings}>
            {result.warnings.map((warning) => (
              <li key={warning.kind}>
                <Notice tone={NoticeTone.Info}>{warningText(warning)}</Notice>
              </li>
            ))}
          </ul>
        )}
        <div className={styles.next}>
          <LinkButton
            href={newDesignFromCapture(capture.id)}
            variant={ButtonVariant.Accent}
            size={ButtonSize.Large}
            withArrow
            transitionTypes={[TransitionType.Forward]}
          >
            {t("capture.next.action")}
          </LinkButton>
          <p className={styles.nextLead}>{t("capture.next.lead")}</p>
        </div>
        <div className={styles.actions}>
          {parallax && capture.mode === CaptureMode.Photo && (
            <LinkButton href={captureLink("trace")} variant={ButtonVariant.Secondary}>
              {t("capture.action.trace")}
            </LinkButton>
          )}
          <LinkButton href={captureLink(capture.mode)} variant={ButtonVariant.Secondary}>
            {t("capture.action.retake")}
          </LinkButton>
          <LinkButton href={captureLink("any")} variant={ButtonVariant.Quiet}>
            {t("capture.action.another")}
          </LinkButton>
        </div>
      </div>
    </div>
  );
}

/** Pending: the photo under a scanning beam, and what vision is doing. */
function Measuring({ capture, slow }: { readonly capture: Capture; readonly slow: boolean }) {
  const [line, setLine] = useState(0);
  useEffect(() => {
    const timer = setInterval(() => setLine((index) => (index + 1) % MEASURING_LINES.length), MEASURING_LINE_MS);
    return () => clearInterval(timer);
  }, []);
  return (
    <div className={styles.layout}>
      <OutlineFigure
        image={photoOf(capture)}
        result={capture.result}
        measuring
        overlay={(view) =>
          capture.tapPoint.kind === "some" && (
            <TapPin view={view} point={capture.tapPoint.value} state={PinState.Sending} />
          )
        }
      />
      <div className={styles.side}>
        <header className={styles.head}>
          <p className={styles.eyebrow}>{t("capture.measuring.eyebrow")}</p>
          <h1 className={styles.title}>
            {t(capture.tapPoint.kind === "some" ? "capture.measuring.tapTitle" : "capture.measuring.title")}
          </h1>
        </header>
        <p className={styles.line} role="status" aria-live="polite" key={line}>
          <span className={styles.dot} aria-hidden="true" />
          {t(MEASURING_LINES[line])}
        </p>
        {slow && <Notice>{t("capture.measuring.slow")}</Notice>}
      </div>
    </div>
  );
}

/** Failed, with no outline to show: what went wrong, how to fix it, and the ways on. */
function Failed({ capture, onCorrected }: { readonly capture: Capture; readonly onCorrected: (capture: Capture) => void }) {
  const [tool, setTool] = useState<FigureTool>(FigureTool.None);
  const error = capture.error;
  const help = error.kind === "some" ? (FAILURES[error.value.code] ?? GENERIC_FAILURE) : GENERIC_FAILURE;
  const canTap = help.tap && photoOf(capture).kind === "some";
  return (
    <div className={styles.layout}>
      <OutlineReview
        capture={capture}
        tool={tool}
        onTool={setTool}
        offerTools={false}
        dimmed
        entrance={OutlineEntrance.Draw}
        onCorrected={({ capture: next }) => onCorrected(next)}
      />
      <div className={styles.side}>
        <header className={styles.head}>
          <span className={styles.failIcon} aria-hidden="true">
            <Icon name="alert" size={26} />
          </span>
          <h1 className={styles.title}>{t(help.title)}</h1>
        </header>
        <Notice tone={NoticeTone.Error}>{failureText(capture)}</Notice>
        {canTap && <p className={styles.lead}>{t("capture.fail.tapLead")}</p>}
        <div className={styles.actions}>
          {canTap && (
            <Button onClick={() => setTool(FigureTool.Tap)} disabled={tool === FigureTool.Tap}>
              <Icon name="target" size={18} />
              {t("capture.action.tapProduct")}
            </Button>
          )}
          {help.retake && (
            <LinkButton
              href={captureLink(capture.mode)}
              variant={canTap ? ButtonVariant.Secondary : ButtonVariant.Primary}
              withArrow={!canTap}
            >
              {t("capture.action.retake")}
            </LinkButton>
          )}
          {help.trace && capture.mode === CaptureMode.Photo && (
            <LinkButton href={captureLink("trace")} variant={ButtonVariant.Secondary}>
              {t("capture.action.trace")}
            </LinkButton>
          )}
          {help.typed && (
            <LinkButton href={captureLink("typed")} variant={ButtonVariant.Quiet}>
              {t("capture.action.typed")}
            </LinkButton>
          )}
        </div>
      </div>
    </div>
  );
}

/**
 * One capture (`/capture/{id}`): measuring while vision works (polled about once a second),
 * then the outline on the photo with its sizes, or what went wrong and how to try again.
 */
export function CaptureView() {
  const { captureId } = useParams<{ captureId: string }>();
  const query = useCapture(captureId);

  if (query.capture.kind === "none") {
    if (query.error.kind === "some" && NOT_HERE.has(query.error.value.code)) {
      return (
        <div className={styles.missing}>
          <h1 className={styles.title}>{t("capture.missing.title")}</h1>
          <p className={styles.lead}>{t("capture.missing.lead")}</p>
          <LinkButton href={captureLink("any")} size={ButtonSize.Large} withArrow>
            {t("capture.action.another")}
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
      <div className={styles.layout} aria-busy="true">
        <Skeleton shape={SkeletonShape.Block} />
        <Skeleton shape={SkeletonShape.Title} count={1} />
      </div>
    );
  }

  const capture = query.capture.value;
  // A later run that failed keeps the last outline: it stays on screen with the reason.
  const measured =
    capture.status === CaptureStatus.Done ||
    (capture.status === CaptureStatus.Failed && capture.result.kind === "some");
  return (
    <div className={styles.page} data-status={capture.status}>
      <LinkButton href="/app" variant={ButtonVariant.Quiet}>
        {t("capture.toWorkspace")}
      </LinkButton>
      {capture.status === CaptureStatus.Pending && <Measuring capture={capture} slow={query.slow} />}
      {measured && <Measured capture={capture} onCorrected={query.apply} />}
      {capture.status === CaptureStatus.Failed && !measured && (
        <Failed capture={capture} onCorrected={query.apply} />
      )}
    </div>
  );
}
