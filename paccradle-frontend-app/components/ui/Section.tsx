import type { ReactNode } from "react";
import { SplitText } from "@/components/motion/SplitText";
import { revealDelay } from "@/components/motion/reveal";
import styles from "./Section.module.css";

/** Background tone of a page section. */
export const SectionTone = {
  /** Oat paper. */
  Plain: "plain",
  /** Sand, with a torn-paper top edge. */
  Muted: "muted",
} as const;

export type SectionTone = (typeof SectionTone)[keyof typeof SectionTone];

/** Props for a titled page section. */
export interface SectionProps {
  /** Anchor id; the heading gets `${id}-title`. */
  readonly id: string;
  readonly title: string;
  /** Small uppercase label above the title. */
  readonly eyebrow?: string;
  readonly lead?: string;
  readonly tone?: SectionTone;
  readonly children: ReactNode;
}

/** Reveal order of the header parts. */
const LEAD_POSITION = 2;

/**
 * A full-width page section. Its eyebrow, title (word by word) and lead animate in when the
 * section scrolls into view.
 */
export function Section({
  id,
  title,
  eyebrow,
  lead,
  tone = SectionTone.Plain,
  children,
}: SectionProps) {
  const headingId = `${id}-title`;
  return (
    <section id={id} aria-labelledby={headingId} className={`${styles.section} ${styles[tone]}`}>
      <header className={styles.header}>
        {eyebrow && (
          <p className={styles.eyebrow} data-reveal>
            {eyebrow}
          </p>
        )}
        <h2 id={headingId} className={styles.title} data-reveal="words">
          <SplitText text={title} />
        </h2>
        {lead && (
          <p className={styles.lead} data-reveal style={revealDelay(LEAD_POSITION)}>
            {lead}
          </p>
        )}
      </header>
      {children}
    </section>
  );
}
