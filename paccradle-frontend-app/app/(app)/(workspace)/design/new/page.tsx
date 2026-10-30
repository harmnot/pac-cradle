import type { Metadata } from "next";
import { Suspense } from "react";
import { PageTransition } from "@/components/motion/PageTransition";
import { Skeleton, SkeletonShape } from "@/components/ui/Skeleton";
import { DesignChoices } from "@/features/design/components/DesignChoices";
import { t } from "@/lib/i18n";

export const metadata: Metadata = { title: t("meta.designNew") };

/** A new design: height, quantity, insert style, board and box, with a live preview (CLAUDE.md §3 steps 5–8). */
export default function Page() {
  return (
    <PageTransition>
      <Suspense fallback={<Skeleton shape={SkeletonShape.Block} count={2} />}>
        <DesignChoices />
      </Suspense>
    </PageTransition>
  );
}
