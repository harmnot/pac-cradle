import type { Metadata } from "next";
import { Suspense } from "react";
import { PageTransition } from "@/components/motion/PageTransition";
import { AddPasskeyPanel } from "@/features/identity/components/AddPasskeyPanel";
import { t } from "@/lib/i18n";
import { AuthFallback } from "@/features/identity/components/AuthFallback";

export const metadata: Metadata = { title: t("meta.passkey") };

/** Add a passkey on this device. */
export default function Page() {
  return (
    <PageTransition>
      <Suspense fallback={<AuthFallback />}>
        <AddPasskeyPanel />
      </Suspense>
    </PageTransition>
  );
}
