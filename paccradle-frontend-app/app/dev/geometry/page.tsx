import type { Metadata } from "next";
import { notFound } from "next/navigation";
import { EngineCheck } from "@/features/design/components/EngineCheck";
import { t } from "@/lib/i18n";
import styles from "./page.module.css";

export const metadata: Metadata = {
  title: t("dev.engine.title"),
  robots: { index: false, follow: false },
};

/** Development only (M1-A10): the browser engine, live and timed. Not served in production. */
export default function EngineCheckPage() {
  if (process.env.NODE_ENV === "production") {
    notFound();
  }
  return (
    <main id="main" className={styles.page}>
      <h1 className={styles.title}>{t("dev.engine.title")}</h1>
      <p className={styles.lead}>{t("dev.engine.lead")}</p>
      <EngineCheck />
    </main>
  );
}
