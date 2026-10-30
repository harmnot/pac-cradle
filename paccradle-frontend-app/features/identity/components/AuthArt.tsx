import type { CSSProperties } from "react";
import { LogoMark } from "@/features/marketing/components/LogoMark";
import { t } from "@/lib/i18n";
import styles from "./AuthArt.module.css";

/** Where the two products sit on the drawn card, and how big they are (viewBox units). */
const HOLES = [
  { cx: 132, cy: 170, r: 38 },
  { cx: 248, cy: 170, r: 38 },
] as const;
/** A fit ring sits this far outside its hole: the clearance, drawn. */
const RING_GAP = 9;

/**
 * The kraft panel beside the sign-in screens: a cutout insert draws itself in (cut solid,
 * crease dashed), two bottles settle into their holes, and their fit rings pulse green.
 * Decorative; the words beside it carry the meaning.
 */
export function AuthArt() {
  return (
    <aside className={styles.panel} aria-label={t("auth.art.label")}>
      <p className={styles.band}>
        <LogoMark />
        <span>{t("auth.art.band")}</span>
      </p>

      <svg viewBox="0 0 380 300" className={styles.drawing} aria-hidden="true">
        <rect className={styles.card} x="30" y="70" width="320" height="200" rx="14" pathLength={1} />
        <path className={styles.crease} d="M30 110H350M30 230H350" pathLength={1} />
        {HOLES.map((hole, index) => (
          <g key={hole.cx} className={styles.slot} style={{ "--slot": index } as CSSProperties}>
            <circle className={styles.ring} cx={hole.cx} cy={hole.cy} r={hole.r + RING_GAP} />
            <circle className={styles.hole} cx={hole.cx} cy={hole.cy} r={hole.r} pathLength={1} />
            <g transform={`translate(${hole.cx} ${hole.cy})`}>
              <g className={styles.bottle}>
                <circle r={hole.r - 6} className={styles.bottleBody} />
                <circle r={hole.r - 22} className={styles.bottleCap} />
              </g>
            </g>
          </g>
        ))}
        <g className={styles.dimension}>
          <path d="M94 40H170M94 32v16M170 32v16" pathLength={1} />
          <text x="132" y="26" textAnchor="middle">
            {t("auth.art.diameter")}
          </text>
        </g>
        <g className={styles.chip}>
          <rect x="224" y="22" width="152" height="30" rx="15" />
          <circle cx="242" cy="37" r="5" />
          <text x="254" y="41">
            {t("auth.art.fit")}
          </text>
        </g>
      </svg>

      <div className={styles.copy}>
        <h2 className={styles.title}>{t("auth.art.title")}</h2>
        <ul className={styles.points}>
          <li>{t("auth.art.point.passkey")}</li>
          <li>{t("auth.art.point.saved")}</li>
          <li>{t("auth.art.point.files")}</li>
        </ul>
      </div>
    </aside>
  );
}
