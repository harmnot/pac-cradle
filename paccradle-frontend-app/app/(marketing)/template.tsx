import type { ReactNode } from "react";
import styles from "./template.module.css";

/** Re-mounts on every navigation, so each marketing page fades in. */
export default function MarketingTemplate({ children }: Readonly<{ children: ReactNode }>) {
  return (
    <main id="main" className={styles.page}>
      {children}
    </main>
  );
}
