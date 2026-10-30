import type { Metadata } from "next";
import { Suspense } from "react";
import { PageTransition } from "@/components/motion/PageTransition";
import { Skeleton, SkeletonShape } from "@/components/ui/Skeleton";
import { CaptureView } from "@/features/capture/components/CaptureView";
import { t } from "@/lib/i18n";

export const metadata: Metadata = { title: t("meta.captureResult") };

/** One capture: measuring, then its outline and sizes, or what went wrong. */
export default function Page() {
  return (
    <PageTransition>
      <Suspense fallback={<Skeleton shape={SkeletonShape.Block} count={2} />}>
        <CaptureView />
      </Suspense>
    </PageTransition>
  );
}
