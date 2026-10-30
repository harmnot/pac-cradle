import { Suspense, type ReactNode } from "react";
import { AppHeader, AppHeaderFallback } from "@/features/identity/components/AppHeader";
import styles from "./layout.module.css";

/** The signed-in app: the header stays, the pages slide beneath it. */
export default function WorkspaceLayout({ children }: Readonly<{ children: ReactNode }>) {
  return (
    <>
      <Suspense fallback={<AppHeaderFallback />}>
        <AppHeader />
      </Suspense>
      <main id="main" className={styles.main}>
        {children}
      </main>
    </>
  );
}
