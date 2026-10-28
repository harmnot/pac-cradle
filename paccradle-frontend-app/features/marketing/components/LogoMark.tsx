import styles from "./LogoMark.module.css";

/** The Paccradle mark: a kraft box with a fold line and a sage leaf. Decorative. */
export function LogoMark() {
  return (
    <svg viewBox="0 0 32 32" width="30" height="30" aria-hidden="true" className={styles.mark}>
      <path className={styles.box} d="M5 13 16 7.5 27 13v11L16 29.5 5 24Z" />
      <path className={styles.crease} d="M5 13l11 5.5L27 13M16 18.5v11" />
      <path className={styles.leaf} d="M16 15.5c-.4-4.6 2.4-8.6 7.6-9.6.5 5-2.5 8.9-7.6 9.6Z" />
      <path className={styles.vein} d="M16.2 15.2c1.7-2.4 3.4-4.3 5.6-6" />
    </svg>
  );
}
