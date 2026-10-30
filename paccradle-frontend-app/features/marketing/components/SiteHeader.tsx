import Link from "next/link";
import { LinkButton } from "@/components/ui/Button";
import { t } from "@/lib/i18n";
import { LogoMark } from "./LogoMark";
import styles from "./SiteHeader.module.css";

/**
 * Fixed site header. Transparent over the hero; after the first scroll it becomes a floating
 * glass bar (MotionController sets `data-scrolled` on <html>).
 */
export function SiteHeader() {
  return (
    <header className={styles.header}>
      <Link href="/" className={styles.brand} aria-label={t("nav.home")}>
        <LogoMark />
        <span className={styles.wordmark}>{t("site.name")}</span>
      </Link>
      <nav aria-label={t("nav.main")}>
        <ul className={styles.links}>
          <li className={styles.wide}>
            <Link href="/#how" className={styles.link}>
              {t("nav.how")}
            </Link>
          </li>
          <li className={styles.wide}>
            <Link href="/#features" className={styles.link}>
              {t("nav.features")}
            </Link>
          </li>
          <li className={styles.medium}>
            <Link href="/pricing" className={styles.link}>
              {t("nav.pricing")}
            </Link>
          </li>
          <li className={styles.medium}>
            <Link href="/sign-in" className={styles.link}>
              {t("nav.signIn")}
            </Link>
          </li>
          <li>
            <LinkButton href="/#join" withArrow>
              {t("nav.join")}
            </LinkButton>
          </li>
        </ul>
      </nav>
    </header>
  );
}
