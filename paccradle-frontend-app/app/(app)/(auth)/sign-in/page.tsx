import type { Metadata } from "next";
import { Suspense } from "react";
import { PageTransition } from "@/components/motion/PageTransition";
import { SignInFlow } from "@/features/identity/components/SignInFlow";
import { t } from "@/lib/i18n";
import { AuthFallback } from "@/features/identity/components/AuthFallback";

export const metadata: Metadata = { title: t("meta.signIn") };

/** Sign in: passkey first, then password or an emailed link. */
export default function Page() {
  return (
    <PageTransition>
      <Suspense fallback={<AuthFallback />}>
        <SignInFlow />
      </Suspense>
    </PageTransition>
  );
}
