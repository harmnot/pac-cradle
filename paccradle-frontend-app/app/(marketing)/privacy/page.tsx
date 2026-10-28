import type { Metadata } from "next";
import { privacyDocument } from "@/content/en/legal";
import { LegalArticle, legalContext } from "@/features/marketing/components/LegalArticle";
import { t } from "@/lib/i18n";

export const metadata: Metadata = { title: t("footer.privacy") };

/** Privacy Policy page. */
export default function PrivacyPage() {
  return <LegalArticle document={privacyDocument(legalContext())} />;
}
