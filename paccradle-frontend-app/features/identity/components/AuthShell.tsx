import Link from "next/link";
import type { ReactNode } from "react";
import { LogoMark } from "@/features/marketing/components/LogoMark";
import { t } from "@/lib/i18n";
import { AuthArt } from "./AuthArt";
import styles from "./AuthShell.module.css";

/**
 * The frame of every sign-in screen: the form on paper, the kraft art panel beside it on wide
 * screens. The panel stays put (`view-transition-name: auth-art`) while the screens slide.
 */
export function AuthShell({ children }: { readonly children: ReactNode }) {
  return (
    <div className={styles.shell}>
      <div className={styles.side}>
        <header className={styles.top}>
          <Link href="/" className={styles.brand} aria-label={t("nav.home")}>
            <LogoMark />
            <span className={styles.wordmark}>{t("site.name")}</span>
          </Link>
          <Link href="/" className={styles.back}>
            {t("auth.backToSite")}
          </Link>
        </header>
        <main id="main" className={styles.main}>
          {children}
        </main>
        <footer className={styles.footer}>
          <Link href="/terms">{t("footer.terms")}</Link>
          <Link href="/privacy">{t("footer.privacy")}</Link>
        </footer>
      </div>
      <AuthArt />
    </div>
  );
}

/** Props for an auth card. */
export interface AuthCardProps {
  /** Small label above the title. */
  readonly eyebrow?: string;
  readonly title: string;
  readonly lead?: ReactNode;
  readonly children: ReactNode;
}

/** One screen's heading and content. */
export function AuthCard({ eyebrow, title, lead, children }: AuthCardProps) {
  return (
    <section className={styles.card} aria-labelledby="auth-title">
      {eyebrow && <p className={styles.eyebrow}>{eyebrow}</p>}
      <h1 id="auth-title" className={styles.title}>
        {title}
      </h1>
      {lead && <p className={styles.lead}>{lead}</p>}
      <div className={styles.body}>{children}</div>
    </section>
  );
}
