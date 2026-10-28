import { SplitText } from "@/components/motion/SplitText";
import { revealDelay } from "@/components/motion/reveal";
import { WaitlistForm } from "@/features/waitlist/components/WaitlistForm";
import { WaitlistTone, type WaitlistSource } from "@/features/waitlist/types";
import { t } from "@/lib/i18n";
import { Botanical } from "./Botanical";
import styles from "./JoinPanel.module.css";

/** Props for the waitlist panel. */
export interface JoinPanelProps {
  /** Where the sign-up came from (sent with the email and to analytics). */
  readonly source: WaitlistSource;
}

const LEAD_POSITION = 2;
const FORM_POSITION = 3;

/** The `#join` section: a forest-green panel with a eucalyptus sprig and the waitlist form. */
export function JoinPanel({ source }: JoinPanelProps) {
  const titleId = "join-title";
  return (
    <section id="join" aria-labelledby={titleId} className={styles.join}>
      <article className={styles.panel} data-reveal>
        <Botanical className={styles.sprig} />
        <header className={styles.header}>
          <p className={styles.eyebrow}>{t("join.eyebrow")}</p>
          <h2 id={titleId} className={styles.title} data-reveal="words">
            <SplitText text={t("join.title")} />
          </h2>
          <p className={styles.lead} data-reveal style={revealDelay(LEAD_POSITION)}>
            {t("join.lead")}
          </p>
        </header>
        <section
          aria-label={t("waitlist.label")}
          className={styles.form}
          data-reveal
          style={revealDelay(FORM_POSITION)}
        >
          <WaitlistForm source={source} tone={WaitlistTone.Dark} />
        </section>
      </article>
    </section>
  );
}
