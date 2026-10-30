import type { Metadata } from "next";
import { PageTransition } from "@/components/motion/PageTransition";
import { AccountOverview } from "@/features/identity/components/AccountOverview";
import { t } from "@/lib/i18n";

export const metadata: Metadata = { title: t("meta.account") };

/** The account: plan and credits, password and passkeys, sign out. */
export default function Page() {
  return (
    <PageTransition>
      <AccountOverview />
    </PageTransition>
  );
}
