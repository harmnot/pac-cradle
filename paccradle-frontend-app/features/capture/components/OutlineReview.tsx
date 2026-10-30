"use client";

import { useId, useMemo, useState } from "react";
import { Button, ButtonVariant } from "@/components/ui/Button";
import { Icon } from "@/components/ui/Icon";
import { IconButton } from "@/components/ui/IconButton";
import { Notice, NoticeTone } from "@/components/ui/Notice";
import { describeError } from "@/lib/errors";
import { t, type MessageKey } from "@/lib/i18n";
import { none, some, type Maybe } from "@/lib/maybe";
import { correctCapture } from "../api";
import { formatMm, sizeLabel } from "../format";
import { MIN_OUTLINE_AREA_MM2, onPaper, paperMapOf, type LiveSize, type PaperMap } from "../geometry";
import { MIN_OUTLINE_POINTS, outlineToSend } from "../outline-editor";
import { shownPhoto } from "../photo-memory";
import type { Capture, Correction, Point } from "../types";
import { OutlineProblem, useOutlineEditing, type OutlineEditing } from "../use-outline-editing";
import { OutlineEditor } from "./OutlineEditor";
import { FigureTool, OutlineFigure, type FigureView, type OutlineEntrance } from "./OutlineFigure";
import { PinState, TapLayer } from "./TapLayer";
import styles from "./OutlineReview.module.css";

/** What a correction did, for the page to follow on. */
export interface Corrected {
  readonly capture: Capture;
  readonly kind: Correction["kind"];
}

/** Props for the review. */
export interface OutlineReviewProps {
  readonly capture: Capture;
  readonly tool: FigureTool;
  readonly onTool: (tool: FigureTool) => void;
  /** Offer both tools under the photo (the measured screen); else only an open tool's bar shows. */
  readonly offerTools: boolean;
  readonly dimmed?: boolean;
  readonly entrance: OutlineEntrance;
  readonly onCorrected: (corrected: Corrected) => void;
}

/** A tap's pin, and why it wasn't sent or didn't work. */
interface TapState {
  readonly pin: Maybe<{ readonly point: Point; readonly state: PinState }>;
  readonly problem: Maybe<string>;
}

const NO_TAP: TapState = { pin: none, problem: none };

const PROBLEM_TEXT: Readonly<Record<OutlineProblem, MessageKey>> = {
  [OutlineProblem.Crosses]: "capture.adjust.crosses",
  [OutlineProblem.TooSmall]: "capture.adjust.tooSmall",
};

/** "≈ ⌀ 52.4 mm" or "≈ 80 × 40.5 mm": the outline's size as drawn. */
function liveLabel(size: LiveSize): string {
  const label =
    size.shape === "round"
      ? sizeLabel({ shape: "round", diameterMm: size.diameterMm })
      : sizeLabel({ shape: "box", lengthMm: size.lengthMm, widthMm: size.widthMm });
  return t("capture.adjust.approx", { size: label });
}

/** The two ways to fix the outline, under the photo. */
function ToolChoices({
  canTap,
  canAdjust,
  onTap,
  onAdjust,
}: {
  readonly canTap: boolean;
  readonly canAdjust: boolean;
  readonly onTap: () => void;
  readonly onAdjust: () => void;
}) {
  return (
    <div className={styles.choices}>
      <p className={styles.prompt}>
        <strong>{t("capture.check.title")}</strong> {t("capture.check.lead")}
      </p>
      <div className={styles.tools}>
        <button type="button" className={styles.tool} onClick={onTap} disabled={!canTap}>
          <span className={styles.toolIcon} aria-hidden="true">
            <Icon name="target" size={22} />
          </span>
          <span className={styles.toolText}>
            <strong>{t("capture.tool.tap")}</strong>
            <span>{t(canTap ? "capture.tool.tapHint" : "capture.tool.tapNoPhoto")}</span>
          </span>
        </button>
        <button type="button" className={styles.tool} onClick={onAdjust} disabled={!canAdjust}>
          <span className={styles.toolIcon} aria-hidden="true">
            <Icon name="nodes" size={22} />
          </span>
          <span className={styles.toolText}>
            <strong>{t("capture.tool.adjust")}</strong>
            <span>{t("capture.tool.adjustHint")}</span>
          </span>
        </button>
      </div>
    </div>
  );
}

/** The bar while tapping: what to do, and a way out. */
function TapBar({ tap, sending, onCancel }: { readonly tap: TapState; readonly sending: boolean; readonly onCancel: () => void }) {
  return (
    <div className={styles.barWrap}>
      <div className={styles.bar} data-tool="tap">
        <p className={styles.instruction} role="status">
          <span className={styles.pulse} aria-hidden="true" />
          {t(sending ? "capture.tap.sending" : "capture.tap.instruction")}
        </p>
        <Button variant={ButtonVariant.Quiet} onClick={onCancel} disabled={sending}>
          {t("capture.tool.cancel")}
        </Button>
      </div>
      {tap.problem.kind === "some" && <Notice tone={NoticeTone.Error}>{tap.problem.value}</Notice>}
    </div>
  );
}

/** The bar while moving points: the size as drawn, history, and keep or cancel. */
function AdjustBar({
  editing,
  saving,
  error,
  onCancel,
  onSave,
}: {
  readonly editing: OutlineEditing;
  readonly saving: boolean;
  readonly error: Maybe<string>;
  readonly onCancel: () => void;
  readonly onSave: () => void;
}) {
  const { state, dispatch, size, problem, changed } = editing;
  const count = state.points.length;
  const keysId = useId();
  return (
    <div className={styles.barWrap}>
      <div className={styles.bar} data-tool="adjust">
        <div className={styles.readout} aria-live="polite">
          {problem.kind === "some" ? (
            <p className={styles.problem}>
              <Icon name="alert" size={18} />
              {t(PROBLEM_TEXT[problem.value], { min: MIN_OUTLINE_AREA_MM2 })}
            </p>
          ) : (
            size.kind === "some" && (
              <p className={styles.size}>
                <span className={styles.sizeLabel}>{t("capture.adjust.asDrawn")}</span>
                <strong>{liveLabel(size.value)}</strong>
                <span className={styles.sizeArea}>
                  {t("capture.size.area", { area: formatMm(size.value.areaMm2 / 100) })}
                </span>
              </p>
            )
          )}
        </div>
        <div className={styles.history} role="toolbar" aria-label={t("capture.adjust.toolbar")} aria-describedby={keysId}>
          <IconButton
            icon="undo"
            label={t("capture.adjust.undo")}
            onClick={() => dispatch({ type: "undo" })}
            disabled={state.past.length === 0}
          />
          <IconButton
            icon="redo"
            label={t("capture.adjust.redo")}
            onClick={() => dispatch({ type: "redo" })}
            disabled={state.future.length === 0}
          />
          <IconButton
            icon="trash"
            label={t("capture.adjust.remove")}
            onClick={() => dispatch({ type: "remove" })}
            disabled={state.selected.kind === "none" || count <= MIN_OUTLINE_POINTS}
          />
          <IconButton
            icon="refresh"
            label={t("capture.adjust.reset")}
            onClick={() => dispatch({ type: "reset" })}
            disabled={!changed}
          />
        </div>
        <div className={styles.commit}>
          <Button variant={ButtonVariant.Quiet} onClick={onCancel} disabled={saving}>
            {t("capture.tool.cancel")}
          </Button>
          <Button onClick={onSave} busy={saving} disabled={!changed || problem.kind === "some"}>
            {t("capture.adjust.save")}
          </Button>
        </div>
        <p className={styles.keys} id={keysId}>
          {t("capture.adjust.howTo")} <span className={styles.keysKeyboard}>{t("capture.adjust.keys")}</span>
        </p>
      </div>
      {error.kind === "some" && <Notice tone={NoticeTone.Error}>{error.value}</Notice>}
      <p className="visually-hidden" aria-live="polite">
        {state.selected.kind === "some" && t("capture.adjust.selected", { n: state.selected.value + 1, count })}
      </p>
    </div>
  );
}

/**
 * The detection review (CLAUDE.md §3 step 4, MILESTONES M1-E3): the photo with its outline,
 * and the two fixes under it. **Tap** the product and vision measures again, keeping the
 * outline under the tap (202, the page polls). **Adjust** the points and the outline is
 * measured exactly as drawn (200). Taps and points are in photo pixels; points stay on the
 * paper, and an outline that crosses itself or is too small isn't sent.
 */
export function OutlineReview({ capture, tool, onTool, offerTools, dimmed = false, entrance, onCorrected }: OutlineReviewProps) {
  const image = shownPhoto({ captureId: capture.id, link: capture.photo.kind === "some" ? some(capture.photo.value.url) : none });
  const result = capture.result;
  const map = useMemo<Maybe<PaperMap>>(() => (result.kind === "some" ? paperMapOf(result.value.paper) : none), [result]);
  const editing = useOutlineEditing({ map, active: tool === FigureTool.Adjust });
  const [tap, setTap] = useState<TapState>(NO_TAP);
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState<Maybe<string>>(none);

  const product = result.kind === "some" ? result.value.products.at(0) : undefined;
  const canTap = image.kind === "some";
  const canAdjust = product !== undefined;
  const sending = tap.pin.kind === "some" && tap.pin.value.state === PinState.Sending;

  const openTap = () => {
    setTap(NO_TAP);
    onTool(FigureTool.Tap);
  };

  const openAdjust = () => {
    if (product === undefined) {
      return;
    }
    editing.dispatch({ type: "begin", points: product.contour_px });
    setSaveError(none);
    onTool(FigureTool.Adjust);
  };

  const close = () => {
    setTap(NO_TAP);
    onTool(FigureTool.None);
  };

  const sendTap = async (point: Point) => {
    if (map.kind === "some" && !onPaper({ map: map.value, point })) {
      setTap({ pin: some({ point, state: PinState.Refused }), problem: some(t("capture.tap.offPaper")) });
      return;
    }
    setTap({ pin: some({ point, state: PinState.Sending }), problem: none });
    const answer = await correctCapture({ id: capture.id, correction: { kind: "tap", point: outlineToSend([point])[0] } });
    if (answer.kind === "error") {
      setTap({ pin: some({ point, state: PinState.Refused }), problem: some(describeError(answer.error)) });
      return;
    }
    onCorrected({ capture: answer.value, kind: "tap" });
  };

  const save = async () => {
    setSaving(true);
    setSaveError(none);
    const answer = await correctCapture({
      id: capture.id,
      correction: { kind: "edit", points: outlineToSend(editing.state.points) },
    });
    setSaving(false);
    if (answer.kind === "error") {
      setSaveError(some(describeError(answer.error)));
      return;
    }
    onCorrected({ capture: answer.value, kind: "edit" });
  };

  const overlay = (view: FigureView) => {
    switch (tool) {
      case FigureTool.Tap:
        return <TapLayer view={view} pin={tap.pin} onTap={(point) => void sendTap(point)} />;
      case FigureTool.Adjust:
        return <OutlineEditor view={view} editing={editing} map={map} />;
      default:
        return null;
    }
  };

  return (
    <div className={styles.review}>
      <OutlineFigure
        image={image}
        result={result}
        measuring={false}
        dimmed={dimmed && tool === FigureTool.None}
        tool={tool}
        entrance={entrance}
        overlay={overlay}
      />
      <section className={styles.dock} aria-label={t("capture.check.label")} data-tool={tool}>
        {tool === FigureTool.None && offerTools && (
          <ToolChoices canTap={canTap} canAdjust={canAdjust} onTap={openTap} onAdjust={openAdjust} />
        )}
        {tool === FigureTool.Tap && <TapBar tap={tap} sending={sending} onCancel={close} />}
        {tool === FigureTool.Adjust && (
          <AdjustBar editing={editing} saving={saving} error={saveError} onCancel={close} onSave={() => void save()} />
        )}
      </section>
    </div>
  );
}
