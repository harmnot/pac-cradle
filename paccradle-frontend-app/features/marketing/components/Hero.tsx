import { RotatingWords } from "@/components/motion/RotatingWords";
import { SplitText } from "@/components/motion/SplitText";
import { ButtonSize, LinkButton } from "@/components/ui/Button";
import { Icon } from "@/components/ui/Icon";
import { FakeDoorButton } from "@/features/waitlist/components/FakeDoorButton";
import { t } from "@/lib/i18n";
import { Botanical } from "./Botanical";
import { HeroVisual } from "./HeroVisual";
import styles from "./Hero.module.css";

/** Hand-drawn underline under the rotating product word. */
const SQUIGGLE_PATH = "M3 11C40 4 72 14 112 8s82-7 122 1 48 3 62-2";

function wordCount(text: string): number {
  return text.split(" ").filter((word) => word !== "").length;
}

/**
 * Landing hero: the promise, told word by word, ending on a rotating product
 * ("…that fits your perfume bottle / candle jar / …"), the beta call to action, the
 * $10 fake door and the animated dieline. Pointer movement drives a light parallax.
 */
export function Hero() {
  const titleId = "hero-title";
  const firstLine = t("hero.title.line1");
  const secondLine = t("hero.title.line2");
  const products = [
    t("hero.rotating.perfume"),
    t("hero.rotating.candle"),
    t("hero.rotating.lipstick"),
    t("hero.rotating.giftSet"),
  ];
  const secondLineStart = wordCount(firstLine);
  const rotatingIndex = secondLineStart + wordCount(secondLine);

  return (
    <section aria-labelledby={titleId} className={styles.hero} data-parallax>
      <Botanical className={styles.sprig} />
      <header className={styles.copy}>
        <p className={styles.eyebrow}>{t("hero.eyebrow")}</p>
        <h1 id={titleId} className={styles.title}>
          <span className="visually-hidden">{t("hero.title")}</span>
          <span className={styles.line}>
            <SplitText text={firstLine} withLabel={false} />
          </span>
          <span className={styles.line}>
            <SplitText text={secondLine} startIndex={secondLineStart} withLabel={false} />
          </span>
          <span className={styles.line}>
            <span className={styles.accent}>
              <RotatingWords words={products} wordIndex={rotatingIndex} />
              <svg
                className={styles.squiggle}
                viewBox="0 0 300 16"
                preserveAspectRatio="none"
                aria-hidden="true"
              >
                <path pathLength={1} d={SQUIGGLE_PATH} />
              </svg>
            </span>
          </span>
        </h1>
        <p className={styles.lead}>{t("hero.lead")}</p>
        <ul className={styles.actions}>
          <li>
            <LinkButton href="#join" size={ButtonSize.Large} withArrow>
              {t("nav.join")}
            </LinkButton>
          </li>
          <li>
            <FakeDoorButton />
          </li>
        </ul>
        <p className={styles.note}>
          <Icon name="leaf" size={16} />
          {t("hero.note")}
        </p>
      </header>
      <HeroVisual />
    </section>
  );
}
