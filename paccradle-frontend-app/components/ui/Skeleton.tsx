import styles from "./Skeleton.module.css";

/** Shapes a placeholder can take. */
export const SkeletonShape = {
  /** A line of text. */
  Line: "line",
  /** A heading. */
  Title: "title",
  /** A card or panel. */
  Block: "block",
} as const;

export type SkeletonShape = (typeof SkeletonShape)[keyof typeof SkeletonShape];

/** Props for a placeholder. */
export interface SkeletonProps {
  readonly shape?: SkeletonShape;
  /** How many to stack. */
  readonly count?: number;
}

/** A shimmering paper placeholder while something loads. Hidden from screen readers. */
export function Skeleton({ shape = SkeletonShape.Line, count = 1 }: SkeletonProps) {
  return (
    <span className={styles.stack} aria-hidden="true">
      {Array.from({ length: count }, (_, index) => (
        <span key={index} className={`${styles.skeleton} ${styles[shape]}`} />
      ))}
    </span>
  );
}
