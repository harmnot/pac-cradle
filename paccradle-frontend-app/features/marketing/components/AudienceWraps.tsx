import { revealDelay } from "@/components/motion/reveal";
import { Icon, type IconName } from "@/components/ui/Icon";
import { t, type MessageKey } from "@/lib/i18n";
import styles from "./AudienceWraps.module.css";

interface Audience {
  readonly id: string;
  readonly tone: "terracotta" | "sage" | "kraft" | "rust";
  readonly icon: IconName;
  readonly title: MessageKey;
  readonly body: MessageKey;
}

const AUDIENCES: readonly Audience[] = [
  {
    id: "brands",
    tone: "terracotta",
    icon: "bottle",
    title: "audience.brands.title",
    body: "audience.brands.body",
  },
  {
    id: "designers",
    tone: "sage",
    icon: "pen",
    title: "audience.designers.title",
    body: "audience.designers.body",
  },
  {
    id: "printers",
    tone: "kraft",
    icon: "printer",
    title: "audience.printers.title",
    body: "audience.printers.body",
  },
  {
    id: "makers",
    tone: "rust",
    icon: "scissors",
    title: "audience.makers.title",
    body: "audience.makers.body",
  },
];

/**
 * "Made for" as wrapped packages (brand reference): coloured paper with a folded envelope
 * flap, a cream label band and a fabric tie. On hover the package lifts and tilts, the flap
 * lifts and the tie slides.
 */
export function AudienceWraps() {
  return (
    <ul className={styles.grid}>
      {AUDIENCES.map((audience, index) => (
        <li key={audience.id} data-reveal style={revealDelay(index)}>
          <article
            className={`${styles.wrap} ${styles[audience.tone]}`}
            aria-labelledby={`audience-${audience.id}`}
          >
            <header className={styles.label}>
              <Icon name={audience.icon} size={18} />
              <h3 id={`audience-${audience.id}`} className={styles.title}>
                {t(audience.title)}
              </h3>
            </header>
            <p className={styles.body}>{t(audience.body)}</p>
          </article>
        </li>
      ))}
    </ul>
  );
}
