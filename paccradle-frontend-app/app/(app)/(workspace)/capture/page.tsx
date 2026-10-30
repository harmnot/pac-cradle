import type { Metadata } from "next";
import { Suspense } from "react";
import { PageTransition } from "@/components/motion/PageTransition";
import { Skeleton, SkeletonShape } from "@/components/ui/Skeleton";
import { CaptureFlow } from "@/features/capture/components/CaptureFlow";
import { t } from "@/lib/i18n";

export const metadata: Metadata = { title: t("meta.capture") };

/** A new capture: how to measure, the paper and tips, then the photo (CLAUDE.md §3 steps 1–3). */
export default function Page() {
  return (
    <PageTransition>
      <Suspense fallback={<Skeleton shape={SkeletonShape.Block} count={2} />}>
        <CaptureFlow />
      </Suspense>
    </PageTransition>
  );
}
