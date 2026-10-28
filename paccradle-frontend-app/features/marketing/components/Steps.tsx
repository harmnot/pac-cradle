import { revealDelay } from "@/components/motion/reveal";
import { Icon, type IconName } from "@/components/ui/Icon";
import { t, type MessageKey } from "@/lib/i18n";
import styles from "./Steps.module.css";

interface Step {
  readonly id: string;
  readonly icon: IconName;
  readonly title: MessageKey;
  readonly body: MessageKey;
}

const STEPS: readonly Step[] = [
  { id: "photo", icon: "camera", title: "how.step1.title", body: "how.step1.body" },
  { id: "insert", icon: "insert", title: "how.step2.title", body: "how.step2.body" },
  { id: "export", icon: "layers", title: "how.step3.title", body: "how.step3.body" },
];

const NUMBER_DIGITS = 2;

/**
 * The three steps, joined by a dashed fold line that draws itself in. Cards follow the
 * pointer with a soft spotlight and lift on hover.
 */
export function Steps() {
  return (
    <ol className={styles.steps} data-reveal="draw">
      {STEPS.map((step, index) => (
        <li key={step.id} className={styles.step} data-reveal data-glow style={revealDelay(index)}>
          <span className={styles.number} aria-hidden="true">
            {String(index + 1).padStart(NUMBER_DIGITS, "0")}
          </span>
          <span className={styles.icon} aria-hidden="true">
            <Icon name={step.icon} size={26} />
          </span>
          <h3 className={styles.title}>{t(step.title)}</h3>
          <p className={styles.body}>{t(step.body)}</p>
        </li>
      ))}
    </ol>
  );
}
