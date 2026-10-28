import { Section, SectionTone } from "@/components/ui/Section";
import { AudienceWraps } from "@/features/marketing/components/AudienceWraps";
import { DemoVideo } from "@/features/marketing/components/DemoVideo";
import { FeatureBento } from "@/features/marketing/components/FeatureBento";
import { Hero } from "@/features/marketing/components/Hero";
import { JoinPanel } from "@/features/marketing/components/JoinPanel";
import { Marquee } from "@/features/marketing/components/Marquee";
import { PlanCards } from "@/features/marketing/components/PlanCards";
import { Steps } from "@/features/marketing/components/Steps";
import { WaitlistSource } from "@/features/waitlist/types";
import { t } from "@/lib/i18n";

/**
 * Landing page (M0-04): the promise and animated dieline, what people pack, how it works,
 * the demo, what you get, who it's for, pricing and the waitlist.
 */
export default function LandingPage() {
  return (
    <>
      <Hero />
      <Marquee />
      <Section id="how" eyebrow={t("how.eyebrow")} title={t("how.title")} lead={t("how.lead")}>
        <Steps />
      </Section>
      <DemoVideo />
      <Section id="features" eyebrow={t("features.eyebrow")} title={t("features.title")}>
        <FeatureBento />
      </Section>
      <Section
        id="audience"
        eyebrow={t("audience.eyebrow")}
        title={t("audience.title")}
        tone={SectionTone.Muted}
      >
        <AudienceWraps />
      </Section>
      <Section
        id="pricing"
        eyebrow={t("pricing.eyebrow")}
        title={t("pricing.title")}
        lead={t("pricing.lead")}
      >
        <PlanCards />
      </Section>
      <JoinPanel source={WaitlistSource.Join} />
    </>
  );
}
