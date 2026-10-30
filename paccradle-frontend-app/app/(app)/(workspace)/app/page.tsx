import type { Metadata } from "next";
import { PageTransition } from "@/components/motion/PageTransition";
import { WorkspaceHome } from "@/features/workspace/components/WorkspaceHome";
import { t } from "@/lib/i18n";

export const metadata: Metadata = { title: t("meta.workspace") };

/** The workspace: what the account has, a new insert, the newest designs. */
export default function Page() {
  return (
    <PageTransition>
      <WorkspaceHome />
    </PageTransition>
  );
}
