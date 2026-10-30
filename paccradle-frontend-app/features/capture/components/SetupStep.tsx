import { useId } from "react";
import { revealDelay } from "@/components/motion/reveal";
import { Button, ButtonSize, ButtonVariant } from "@/components/ui/Button";
import { Icon } from "@/components/ui/Icon";
import { Paper } from "@/features/identity/types";
import { t, type MessageKey } from "@/lib/i18n";
import { CaptureMode, PEN_WIDTHS_MM } from "../types";
import { Sketch, type SketchName } from "./Sketch";
import styles from "./SetupStep.module.css";

interface Tip {
  readonly sketch: SketchName;
  readonly title: MessageKey;
  readonly body: MessageKey;
}

/** The capture rules shown before the photo (CLAUDE.md §5.1), per mode. */
const TIPS: Readonly<Record<CaptureMode, readonly Tip[]>> = {
  [CaptureMode.Photo]: [
    { sketch: "corners", title: "capture.tip.corners.title", body: "capture.tip.corners.body" },
    { sketch: "centre", title: "capture.tip.centre.title", body: "capture.tip.centre.body" },
    { sketch: "height", title: "capture.tip.height.title", body: "capture.tip.height.body" },
    { sketch: "light", title: "capture.tip.light.title", body: "capture.tip.light.body" },
    { sketch: "colour", title: "capture.tip.colour.title", body: "capture.tip.colour.body" },
  ],
  [CaptureMode.Trace]: [
    { sketch: "trace", title: "capture.tip.trace.title", body: "capture.tip.trace.body" },
    { sketch: "remove", title: "capture.tip.remove.title", body: "capture.tip.remove.body" },
    { sketch: "corners", title: "capture.tip.corners.title", body: "capture.tip.corners.body" },
    { sketch: "height", title: "capture.tip.height.title", body: "capture.tip.traceHeight.body" },
  ],
};

/** The two sheets, drawn to scale (height ÷ width), with their sizes. */
const SHEETS: ReadonlyArray<{ readonly paper: Paper; readonly ratio: number; readonly name: MessageKey; readonly size: MessageKey }> = [
  { paper: Paper.A4, ratio: 297 / 210, name: "capture.paper.a4", size: "capture.paper.a4.size" },
  { paper: Paper.Letter, ratio: 11 / 8.5, name: "capture.paper.letter", size: "capture.paper.letter.size" },
];

/** Props for the setup step. */
export interface SetupStepProps {
  readonly mode: CaptureMode;
  readonly paper: Paper;
  readonly penWidthMm: number;
  readonly onPaper: (paper: Paper) => void;
  readonly onPenWidth: (width: number) => void;
  readonly onReady: () => void;
  readonly onBack: () => void;
}

/** Step 2: the paper (defaulted by region, switchable), how to take the photo, the pen. */
export function SetupStep({ mode, paper, penWidthMm, onPaper, onPenWidth, onReady, onBack }: SetupStepProps) {
  const paperLabel = useId();
  const penLabel = useId();
  return (
    <section className={styles.step} aria-labelledby="setup-title">
      <header className={styles.head}>
        <h1 id="setup-title" className={styles.title}>
          {mode === CaptureMode.Trace ? t("capture.setup.traceTitle") : t("capture.setup.photoTitle")}
        </h1>
        <p className={styles.lead}>{t("capture.setup.lead")}</p>
      </header>

      <div className={styles.block}>
        <h2 id={paperLabel} className={styles.blockTitle}>
          {t("capture.paper.title")}
        </h2>
        <div role="radiogroup" aria-labelledby={paperLabel} className={styles.sheets}>
          {SHEETS.map((sheet) => (
            <button
              key={sheet.paper}
              type="button"
              role="radio"
              aria-checked={paper === sheet.paper}
              className={styles.sheet}
              onClick={() => onPaper(sheet.paper)}
            >
              <span className={styles.sheetArt} aria-hidden="true">
                <span className={styles.sheetPaper} style={{ aspectRatio: `1 / ${sheet.ratio}` }} />
              </span>
              <span className={styles.sheetText}>
                <strong>{t(sheet.name)}</strong>
                <span>{t(sheet.size)}</span>
              </span>
              <span className={styles.check} aria-hidden="true">
                <Icon name="check" size={14} />
              </span>
            </button>
          ))}
        </div>
        <p className={styles.note}>{t("capture.paper.note")}</p>
      </div>

      {mode === CaptureMode.Trace && (
        <div className={styles.block}>
          <h2 id={penLabel} className={styles.blockTitle}>
            {t("capture.pen.title")}
          </h2>
          <div role="radiogroup" aria-labelledby={penLabel} className={styles.chips}>
            {PEN_WIDTHS_MM.map((width) => (
              <button
                key={width}
                type="button"
                role="radio"
                aria-checked={penWidthMm === width}
                className={styles.chip}
                onClick={() => onPenWidth(width)}
              >
                <span className={styles.line} style={{ height: `${Math.max(width * 3, 1)}px` }} aria-hidden="true" />
                {t("capture.pen.width", { mm: width.toFixed(1) })}
              </button>
            ))}
          </div>
          <p className={styles.note}>{t("capture.pen.note")}</p>
        </div>
      )}

      <div className={styles.block}>
        <h2 className={styles.blockTitle}>{t("capture.tips.title")}</h2>
        <ol className={styles.tips}>
          {TIPS[mode].map((tip, index) => (
            <li key={tip.title} className={styles.tip} style={revealDelay(index, 80)}>
              <Sketch name={tip.sketch} className={styles.tipArt} />
              <span className={styles.tipText}>
                <strong>{t(tip.title)}</strong>
                <span>{t(tip.body)}</span>
              </span>
            </li>
          ))}
        </ol>
      </div>

      <div className={styles.actions}>
        <Button size={ButtonSize.Large} withArrow onClick={onReady}>
          {t("capture.setup.ready")}
        </Button>
        <Button variant={ButtonVariant.Quiet} onClick={onBack}>
          {t("capture.back")}
        </Button>
      </div>
    </section>
  );
}
