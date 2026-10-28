import type { Metadata } from "next";
import { termsDocument } from "@/content/en/legal";
import { LegalArticle, legalContext } from "@/features/marketing/components/LegalArticle";
import { t } from "@/lib/i18n";

export const metadata: Metadata = { title: t("footer.terms") };

/** Terms & Conditions page. */
export default function TermsPage() {
  return <LegalArticle document={termsDocument(legalContext())} />;
}
