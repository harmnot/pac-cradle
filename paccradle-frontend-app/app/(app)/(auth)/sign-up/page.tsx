import type { Metadata } from "next";
import { Suspense } from "react";
import { PageTransition } from "@/components/motion/PageTransition";
import { SignUpForm } from "@/features/identity/components/SignUpForm";
import { t } from "@/lib/i18n";
import { AuthFallback } from "@/features/identity/components/AuthFallback";

export const metadata: Metadata = { title: t("meta.signUp") };

/** Create an account with an email link. */
export default function Page() {
  return (
    <PageTransition>
      <Suspense fallback={<AuthFallback />}>
        <SignUpForm />
      </Suspense>
    </PageTransition>
  );
}
