import type { Metadata } from "next";
import { refundDocument } from "@/content/en/legal";
import { LegalArticle, legalContext } from "@/features/marketing/components/LegalArticle";
import { t } from "@/lib/i18n";

export const metadata: Metadata = { title: t("footer.refund") };

/** Refund Policy page. */
export default function RefundPage() {
  return <LegalArticle document={refundDocument(legalContext())} />;
}
