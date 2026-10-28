import Link from "next/link";
import { SplitText } from "@/components/motion/SplitText";
import { COPYRIGHT_YEAR, publicConfig } from "@/lib/config";
import { t } from "@/lib/i18n";
import { valueOr } from "@/lib/maybe";
import { Botanical } from "./Botanical";
import styles from "./SiteFooter.module.css";

/**
 * Forest-green site footer with a large wordmark. The legal links live here so Terms,
 * Refund and Privacy stay reachable from every page (Paddle domain approval, CLAUDE.md §9.1).
 */
export function SiteFooter() {
  const owner = valueOr(publicConfig.legalName, t("site.name"));
  return (
    <footer className={styles.footer}>
      <Botanical className={styles.sprig} />
      <p className={styles.wordmark} aria-hidden="true" data-reveal="words">
        <SplitText text={t("site.name")} withLabel={false} />
      </p>
      <p className={styles.tagline}>{t("footer.tagline")}</p>
      <nav aria-label={t("nav.footer")} className={styles.nav}>
        <section aria-labelledby="footer-product">
          <h2 id="footer-product" className={styles.heading}>
            {t("footer.product")}
          </h2>
          <ul className={styles.links}>
            <li>
              <Link href="/#how">{t("nav.how")}</Link>
            </li>
            <li>
              <Link href="/#features">{t("nav.features")}</Link>
            </li>
            <li>
              <Link href="/pricing">{t("footer.pricing")}</Link>
            </li>
            <li>
              <Link href="/#join">{t("nav.join")}</Link>
            </li>
          </ul>
        </section>
        <section aria-labelledby="footer-legal">
          <h2 id="footer-legal" className={styles.heading}>
            {t("footer.legalTitle")}
          </h2>
          <ul className={styles.links}>
            <li>
              <Link href="/terms">{t("footer.terms")}</Link>
            </li>
            <li>
              <Link href="/refund">{t("footer.refund")}</Link>
            </li>
            <li>
              <Link href="/privacy">{t("footer.privacy")}</Link>
            </li>
          </ul>
        </section>
      </nav>
      <p className={`${styles.note} ${styles.divider}`}>{t("footer.paddle")}</p>
      <p className={styles.note}>{t("footer.copyright", { year: COPYRIGHT_YEAR, owner })}</p>
    </footer>
  );
}
