import type { ReactNode } from "react";
import { MotionController } from "@/components/motion/MotionController";
import { SiteFooter } from "@/features/marketing/components/SiteFooter";
import { SiteHeader } from "@/features/marketing/components/SiteHeader";

/**
 * Marketing pages share the floating header, the footer with the legal links and the
 * motion controller. `<main>` lives in template.tsx so every page animates in.
 */
export default function MarketingLayout({ children }: Readonly<{ children: ReactNode }>) {
  return (
    <>
      <SiteHeader />
      {children}
      <SiteFooter />
      <MotionController />
    </>
  );
}
