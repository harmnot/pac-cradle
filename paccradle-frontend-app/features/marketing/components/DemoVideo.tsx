import { Icon } from "@/components/ui/Icon";
import { Section, SectionTone } from "@/components/ui/Section";
import { t } from "@/lib/i18n";
import styles from "./DemoVideo.module.css";

/**
 * Demo video slot (M0-04), framed as a sage cutting mat. Shows a placeholder until the demo
 * video from M0-13 exists; then replace the play badge and caption with a `<video>`.
 */
export function DemoVideo() {
  return (
    <Section
      id="demo"
      eyebrow={t("demo.eyebrow")}
      title={t("demo.title")}
      lead={t("demo.lead")}
      tone={SectionTone.Muted}
    >
      <figure className={styles.frame} data-reveal>
        <span className={styles.play} aria-hidden="true">
          <Icon name="play" size={30} />
        </span>
        <figcaption className={styles.caption}>{t("demo.placeholder")}</figcaption>
      </figure>
    </Section>
  );
}
