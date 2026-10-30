import { Skeleton, SkeletonShape } from "@/components/ui/Skeleton";
import styles from "./AuthFallback.module.css";

/** What an auth screen shows while it starts in the browser. */
export function AuthFallback() {
  return (
    <div className={styles.fallback}>
      <Skeleton shape={SkeletonShape.Title} />
      <Skeleton count={2} />
      <Skeleton shape={SkeletonShape.Block} />
    </div>
  );
}
