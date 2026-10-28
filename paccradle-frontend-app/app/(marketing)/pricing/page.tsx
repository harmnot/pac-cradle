import type { Metadata } from "next";
import { Section, SectionTone } from "@/components/ui/Section";
import { Faq } from "@/features/marketing/components/Faq";
import { JoinPanel } from "@/features/marketing/components/JoinPanel";
import { PlanCards } from "@/features/marketing/components/PlanCards";
import { WaitlistSource } from "@/features/waitlist/types";
import { t } from "@/lib/i18n";
import styles from "./page.module.css";

export const metadata: Metadata = {
  title: t("nav.pricing"),
  description: t("pricing.lead"),
};

/** Pricing page (required for Paddle domain approval, CLAUDE.md §9.1). */
export default function PricingPage() {
  return (
    <>
      <Section
        id="plans"
        eyebrow={t("pricing.eyebrow")}
        title={t("pricing.title")}
        lead={t("pricing.lead")}
      >
        <PlanCards />
        <p className={styles.note}>{t("pricing.note")}</p>
      </Section>
      <Section id="faq" eyebrow={t("faq.eyebrow")} title={t("faq.title")} tone={SectionTone.Muted}>
        <Faq />
      </Section>
      <JoinPanel source={WaitlistSource.Pricing} />
    </>
  );
}
