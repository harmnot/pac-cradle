import { Icon } from "@/components/ui/Icon";
import { t, type MessageKey } from "@/lib/i18n";
import styles from "./Marquee.module.css";

const PRODUCT_KEYS: readonly MessageKey[] = [
  "marquee.perfume",
  "marquee.candles",
  "marquee.skincare",
  "marquee.tea",
  "marquee.jewelry",
  "marquee.giftSets",
  "marquee.cosmetics",
  "marquee.gadgets",
];

const MATERIAL_KEYS: readonly MessageKey[] = [
  "marquee.paperboard",
  "marquee.chipboard",
  "marquee.eFlute",
  "marquee.bFlute",
  "marquee.cricut",
  "marquee.laser",
  "marquee.cnc",
  "marquee.printShops",
];

/** How many times the words repeat in a band; the copies after the first are hidden from screen readers. */
const COPIES = [0, 1] as const;

interface BandProps {
  readonly label: string;
  readonly keys: readonly MessageKey[];
  readonly tone: "terracotta" | "sage";
}

/** One tape band: the words twice in a row, sliding by one copy for an endless loop. */
function Band({ label, keys, tone }: BandProps) {
  return (
    <section aria-label={label} className={`${styles.band} ${styles[tone]}`}>
      <ul className={styles.track}>
        {COPIES.flatMap((copy) =>
          keys.map((key) => (
            <li
              key={`${copy}-${key}`}
              className={styles.item}
              {...(copy > 0 ? { "aria-hidden": true } : {})}
            >
              {t(key)}
              <Icon name="leaf" size={18} className={styles.mark} />
            </li>
          )),
        )}
      </ul>
    </section>
  );
}

/**
 * Two crossing tape bands, like the ties on wrapped packages: what people pack, and what
 * they cut it with. Hovering pauses them; reduced motion keeps them still.
 */
export function Marquee() {
  return (
    <aside aria-label={t("marquee.label")} className={styles.cross}>
      <Band label={t("marquee.productsLabel")} keys={PRODUCT_KEYS} tone="terracotta" />
      <Band label={t("marquee.materialsLabel")} keys={MATERIAL_KEYS} tone="sage" />
    </aside>
  );
}
