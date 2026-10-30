import type { Metadata } from "next";
import { Suspense } from "react";
import { PageTransition } from "@/components/motion/PageTransition";
import { SetPasswordForm } from "@/features/identity/components/SetPasswordForm";
import { t } from "@/lib/i18n";
import { AuthFallback } from "@/features/identity/components/AuthFallback";

export const metadata: Metadata = { title: t("meta.password") };

/** Set or change the account's password. */
export default function Page() {
  return (
    <PageTransition>
      <Suspense fallback={<AuthFallback />}>
        <SetPasswordForm />
      </Suspense>
    </PageTransition>
  );
}
