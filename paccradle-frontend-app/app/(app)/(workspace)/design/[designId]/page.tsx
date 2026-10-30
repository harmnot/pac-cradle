import type { Metadata } from "next";
import { Suspense } from "react";
import { PageTransition } from "@/components/motion/PageTransition";
import { Skeleton, SkeletonShape } from "@/components/ui/Skeleton";
import { SavedDesignView } from "@/features/design/components/SavedDesignView";
import { t } from "@/lib/i18n";

export const metadata: Metadata = { title: t("meta.design") };

/** A saved design: what it is and its preview (the editor joins it in M1-E5). */
export default function Page() {
  return (
    <PageTransition>
      <Suspense fallback={<Skeleton shape={SkeletonShape.Block} count={2} />}>
        <SavedDesignView />
      </Suspense>
    </PageTransition>
  );
}
