import {
  LEGAL_LAST_UPDATED,
  type LegalContext,
  type LegalDocument,
} from "@/content/en/legal";
import { publicConfig } from "@/lib/config";
import { formatDate, t } from "@/lib/i18n";
import { valueOr } from "@/lib/maybe";
import styles from "./LegalArticle.module.css";

/** Operator details from config, with visible placeholders when they aren't set yet. */
export function legalContext(): LegalContext {
  return {
    siteName: t("site.name"),
    legalName: valueOr(publicConfig.legalName, t("legal.missingName")),
    contactEmail: valueOr(publicConfig.contactEmail, t("legal.missingEmail")),
  };
}

/** Props for a rendered legal document. */
export interface LegalArticleProps {
  readonly document: LegalDocument;
}

/** Renders a legal document as an article with numbered sections. */
export function LegalArticle({ document }: LegalArticleProps) {
  return (
    <article className={styles.article}>
      <header className={styles.header}>
        <h1 className={styles.title}>{document.title}</h1>
        <p className={styles.updated}>
          {t("legal.updated", { date: formatDate(LEGAL_LAST_UPDATED) })}
        </p>
      </header>
      {document.sections.map((section) => (
        <section key={section.heading} className={styles.section}>
          <h2 className={styles.heading}>{section.heading}</h2>
          {section.paragraphs.map((paragraph) => (
            <p key={paragraph}>{paragraph}</p>
          ))}
        </section>
      ))}
    </article>
  );
}
