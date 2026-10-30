import styles from "./Sketch.module.css";

/**
 * Small line drawings for the capture screens (96 × 72): paper in kraft, the product in sage,
 * guides dashed in terracotta. Lines draw themselves in; decorative only.
 */
const SKETCHES = {
  /** A phone above a sheet with a jar on it (photo mode). */
  photo: (
    <>
      <path className={styles.paper} d="M14 50 30 30h52L66 50Z" pathLength={1} />
      <ellipse className={styles.product} cx="48" cy="40" rx="8" ry="4.5" />
      <rect className={styles.ink} x="38" y="4" width="20" height="12" rx="2.5" pathLength={1} />
      <path className={styles.guide} d="M42 16 40 34M54 16l2 18" pathLength={1} />
    </>
  ),
  /** A pen tracing round where the product stood (trace mode). */
  trace: (
    <>
      <path className={styles.paper} d="M10 56 26 32h60L70 56Z" pathLength={1} />
      <ellipse className={styles.traced} cx="46" cy="44" rx="13" ry="6.5" pathLength={1} />
      <path className={styles.pen} d="M58 42 76 12l5 3-18 30-4 2Z" pathLength={1} />
    </>
  ),
  /** A calliper reading a size (type the sizes). */
  typed: (
    <>
      <path className={styles.ink} d="M12 22h72v10H12Z" pathLength={1} />
      <path className={styles.guide} d="M20 22v5M28 22v3M36 22v5M44 22v3M52 22v5M60 22v3M68 22v5M76 22v3" pathLength={1} />
      <path className={styles.ink} d="M24 32v26h6V32M62 32v26h6V32" pathLength={1} />
      <rect className={styles.product} x="31" y="40" width="30" height="16" rx="3" />
    </>
  ),
  /** The whole sheet, four corners showing, on a darker table. */
  corners: (
    <>
      <rect className={styles.table} x="6" y="6" width="84" height="60" rx="6" />
      <path className={styles.paper} d="M24 14h48v44H24Z" pathLength={1} />
      <path className={styles.guide} d="M20 18v-8h8M76 18v-8h-8M20 54v8h8M76 54v8h-8" pathLength={1} />
    </>
  ),
  /** The product in the middle, clear of the edges. */
  centre: (
    <>
      <path className={styles.paper} d="M22 10h52v52H22Z" pathLength={1} />
      <circle className={styles.product} cx="48" cy="36" r="10" />
      <path className={styles.guide} d="M22 36h16M58 36h16M48 10v16M48 46v16" pathLength={1} />
    </>
  ),
  /** The phone high and level, straight above. */
  height: (
    <>
      <path className={styles.paper} d="M18 62h60" pathLength={1} />
      <rect className={styles.product} x="40" y="46" width="16" height="16" rx="3" />
      <rect className={styles.ink} x="34" y="6" width="28" height="6" rx="2" pathLength={1} />
      <path className={styles.guide} d="M48 12v32M28 9h4M64 9h4" pathLength={1} />
    </>
  ),
  /** Soft light from the side. */
  light: (
    <>
      <path className={styles.ink} d="M10 10h22v30H10ZM21 10v30M10 25h22" pathLength={1} />
      <path className={styles.guide} d="M34 22 54 34M34 30 52 40M34 38l16 8" pathLength={1} />
      <path className={styles.paper} d="M40 60 50 48h36L76 60Z" pathLength={1} />
      <ellipse className={styles.product} cx="63" cy="54" rx="7" ry="3.5" />
    </>
  ),
  /** A white product on coloured paper. */
  colour: (
    <>
      <path className={styles.colouredPaper} d="M14 56 30 30h52L66 56Z" pathLength={1} />
      <ellipse className={styles.white} cx="48" cy="43" rx="11" ry="6" />
    </>
  ),
  /** The product lifted away, its traced line left on the paper. */
  remove: (
    <>
      <path className={styles.paper} d="M10 62 24 42h62L72 62Z" pathLength={1} />
      <ellipse className={styles.traced} cx="48" cy="52" rx="13" ry="6" pathLength={1} />
      <rect className={styles.product} x="38" y="6" width="20" height="22" rx="4" />
      <path className={styles.guide} d="M48 30v10M44 36l4 4 4-4" pathLength={1} />
    </>
  ),
} as const;

/** Names of the drawings. */
export type SketchName = keyof typeof SKETCHES;

/** A small line drawing. */
export function Sketch({ name, className = "" }: { readonly name: SketchName; readonly className?: string }) {
  return (
    <svg viewBox="0 0 96 72" aria-hidden="true" className={`${styles.sketch} ${className}`}>
      {SKETCHES[name]}
    </svg>
  );
}
