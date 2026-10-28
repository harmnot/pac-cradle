import type { CSSProperties } from "react";
import { CountUp } from "@/components/motion/CountUp";
import { Icon } from "@/components/ui/Icon";
import { t } from "@/lib/i18n";
import { EXPORT_FORMATS, HERO_EXAMPLE_DIAMETER_MM } from "../product";
import styles from "./HeroVisual.module.css";

/** Detected paper corners, drawn as rounded brackets just outside the sheet. */
const CORNER_PATHS = [
  "M10 46V22a12 12 0 0 1 12-12h24",
  "M434 10h24a12 12 0 0 1 12 12v24",
  "M470 354v24a12 12 0 0 1-12 12h-24",
  "M46 390H22a12 12 0 0 1-12-12v-24",
] as const;

/**
 * Cradle insert, unfolded (same geometry as CLAUDE.md §6.2): a deck with four walls.
 * Each wall has a filled board shape and its own cut line, so it can fold on its crease.
 */
const WALLS = [
  {
    id: "top",
    fill: "M120 100V40H226A14 14 0 0 0 254 40H360V100Z",
    cut: "M120 100V40H226A14 14 0 0 0 254 40H360V100",
  },
  { id: "right", fill: "M360 100H420V260H360Z", cut: "M360 100H420V260H360" },
  { id: "bottom", fill: "M120 260H360V320H120Z", cut: "M360 260V320H120V260" },
  { id: "left", fill: "M120 100H60V260H120Z", cut: "M120 100H60V260H120" },
] as const;

const CREASE_PATH = "M120 100H360M120 260H360M120 100V260M360 100V260";

/** Bottle holes on the deck with their fit-check result (CLAUDE.md §7.3). */
const HOLES = [
  { x: 170, fit: "ok" },
  { x: 240, fit: "ok" },
  { x: 310, fit: "tight" },
] as const;
const HOLE_Y = 180;
const HOLE_RADIUS = 26;
const RING_RADIUS = 31;

/** The read-out starts counting once the scan has passed. */
const READOUT_DELAY_MS = 1700;
const READOUT_DECIMALS = 1;

const indexStyle = (name: string, index: number): CSSProperties =>
  ({ [`--${name}`]: index }) as CSSProperties;

/**
 * Hero illustration: a paper sheet is detected (corner brackets, scan line), the cradle
 * dieline draws itself, holes pop in with fit rings, and the walls fold up and down in a
 * loop. Floating chips show the measurement, the fit and the file formats.
 */
export function HeroVisual() {
  const titleId = "hero-visual-title";
  return (
    <figure className={styles.visual}>
      <svg viewBox="0 0 480 400" role="img" aria-labelledby={titleId} className={styles.sheet}>
        <title id={titleId}>{t("hero.illustration")}</title>
        <defs>
          <linearGradient id="hero-scan-gradient" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" className={styles.scanStopClear} />
            <stop offset="85%" className={styles.scanStopTint} />
            <stop offset="100%" className={styles.scanStopEdge} />
          </linearGradient>
        </defs>

        <rect className={styles.paper} x="16" y="16" width="448" height="368" rx="12" />
        {CORNER_PATHS.map((path, index) => (
          <path key={path} className={styles.corner} d={path} style={indexStyle("corner-index", index)} />
        ))}

        <g transform="translate(0 20)">
          <rect className={styles.deck} x="120" y="100" width="240" height="160" />
          {WALLS.map((wall) => (
            <g key={wall.id} className={`${styles.wall} ${styles[wall.id]}`}>
              <path className={styles.wallFill} d={wall.fill} />
              <path className={styles.cut} pathLength={1} d={wall.cut} />
            </g>
          ))}
          <path className={styles.crease} d={CREASE_PATH} />
          {HOLES.map((hole, index) => (
            <g key={hole.x} style={indexStyle("hole-index", index)}>
              <circle className={`${styles.ripple} ${styles[hole.fit]}`} cx={hole.x} cy={HOLE_Y} r={RING_RADIUS} />
              <circle className={`${styles.ring} ${styles[hole.fit]}`} cx={hole.x} cy={HOLE_Y} r={RING_RADIUS} />
              <circle className={styles.hole} cx={hole.x} cy={HOLE_Y} r={HOLE_RADIUS} />
            </g>
          ))}
          <path className={styles.dimension} d="M284 180H336M284 174V186M336 174V186" />
          <text className={styles.dimensionLabel} x="310" y="232" textAnchor="middle">
            {t("hero.diameter")}
          </text>
        </g>

        <rect className={styles.scan} x="16" y="16" width="448" height="72" fill="url(#hero-scan-gradient)" />
      </svg>

      <ul className={styles.chips} aria-hidden="true">
        <li className={`${styles.chip} ${styles.readout}`}>
          <span className={styles.chipLabel}>{t("hero.chip.measured")}</span>
          <strong className={styles.chipValue}>
            {t("units.diameter")}{" "}
            <CountUp
              value={HERO_EXAMPLE_DIAMETER_MM}
              decimals={READOUT_DECIMALS}
              delayMs={READOUT_DELAY_MS}
            />{" "}
            {t("units.mm")}
          </strong>
        </li>
        <li className={`${styles.chip} ${styles.fitChip}`}>
          <span className={styles.chipLabel}>{t("hero.chip.fit")}</span>
          <strong className={styles.chipValue}>{t("hero.chip.fitValue")}</strong>
        </li>
        <li className={`${styles.chip} ${styles.files}`}>
          <span className={styles.chipLabel}>
            <Icon name="file" size={14} /> {t("hero.chip.files")}
          </span>
          <strong className={styles.chipValue}>{EXPORT_FORMATS.join(" · ")}</strong>
        </li>
      </ul>

      <figcaption className={styles.label}>
        <Icon name="leaf" size={14} />
        {t("hero.caption")}
      </figcaption>
    </figure>
  );
}
