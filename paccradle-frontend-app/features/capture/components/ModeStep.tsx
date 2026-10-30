import { Icon } from "@/components/ui/Icon";
import { t, type MessageKey } from "@/lib/i18n";
import { Sketch, type SketchName } from "./Sketch";
import styles from "./ModeStep.module.css";

/** The ways to measure a product (CLAUDE.md §3 step 1). */
export const FlowMode = {
  Photo: "photo",
  Trace: "trace",
  Typed: "typed",
} as const;

export type FlowMode = (typeof FlowMode)[keyof typeof FlowMode];

interface ModeCard {
  readonly mode: FlowMode;
  readonly sketch: SketchName;
  readonly badge: MessageKey;
  readonly title: MessageKey;
  readonly body: MessageKey;
}

const CARDS: readonly ModeCard[] = [
  { mode: FlowMode.Photo, sketch: "photo", badge: "capture.mode.photo.badge", title: "capture.mode.photo.title", body: "capture.mode.photo.body" },
  { mode: FlowMode.Trace, sketch: "trace", badge: "capture.mode.trace.badge", title: "capture.mode.trace.title", body: "capture.mode.trace.body" },
  { mode: FlowMode.Typed, sketch: "typed", badge: "capture.mode.typed.badge", title: "capture.mode.typed.title", body: "capture.mode.typed.body" },
];

/** Step 1: how to measure, as three cards that lift and draw on hover. */
export function ModeStep({ onChoose }: { readonly onChoose: (mode: FlowMode) => void }) {
  return (
    <section className={styles.step} aria-labelledby="mode-title">
      <header className={styles.head}>
        <h1 id="mode-title" className={styles.title}>
          {t("capture.mode.title")}
        </h1>
        <p className={styles.lead}>{t("capture.mode.lead")}</p>
      </header>
      <ul className={styles.cards}>
        {CARDS.map((card) => (
          <li key={card.mode}>
            <button type="button" className={styles.card} onClick={() => onChoose(card.mode)}>
              <span className={styles.badge}>{t(card.badge)}</span>
              <Sketch name={card.sketch} className={styles.art} />
              <span className={styles.cardTitle}>{t(card.title)}</span>
              <span className={styles.cardBody}>{t(card.body)}</span>
              <span className={styles.go} aria-hidden="true">
                <Icon name="arrow" size={18} />
              </span>
            </button>
          </li>
        ))}
      </ul>
    </section>
  );
}
