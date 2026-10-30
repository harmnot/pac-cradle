import type { Metadata } from "next";
import { Suspense } from "react";
import { PageTransition } from "@/components/motion/PageTransition";
import { LinkSignIn } from "@/features/identity/components/LinkSignIn";
import { t } from "@/lib/i18n";
import { AuthFallback } from "@/features/identity/components/AuthFallback";

export const metadata: Metadata = { title: t("meta.link") };

/** Where an emailed sign-in or confirmation link lands. */
export default function Page() {
  return (
    <PageTransition>
      <Suspense fallback={<AuthFallback />}>
        <LinkSignIn />
      </Suspense>
    </PageTransition>
  );
}
