import type { CSSProperties, ReactNode } from "react";
import { revealDelay } from "@/components/motion/reveal";
import { Icon, type IconName } from "@/components/ui/Icon";
import { t, type MessageKey } from "@/lib/i18n";
import { EXPORT_FORMATS } from "../product";
import styles from "./FeatureBento.module.css";

const indexStyle = (index: number): CSSProperties => ({ "--item-index": index }) as CSSProperties;

/** Thumbnails of the three MVP insert styles (CLAUDE.md §6.2). */
function InsertStyles() {
  const thumbnails = [
    {
      id: "partition",
      label: t("features.inserts.partition"),
      paths: ["M8 8h80v56H8Z", "M34.7 8v56", "M61.3 8v56", "M8 36h80"],
    },
    {
      id: "cutout",
      label: t("features.inserts.cutout"),
      paths: [
        "M8 8h80v56H8Z",
        "M22 22h14a7 7 0 0 1 0 14H22a7 7 0 0 1 0-14Z",
        "M22 44h44a6 6 0 0 1 0 12H22a6 6 0 0 1 0-12Z",
        "M42 8a6 6 0 0 0 12 0",
      ],
    },
    {
      id: "cradle",
      label: t("features.inserts.cradle"),
      paths: [
        "M30 8h36v16h16v24H66v16H30V48H14V24h16Z",
        "M41 36a7 7 0 1 1 14 0 7 7 0 0 1-14 0Z",
      ],
    },
  ];
  return (
    <ul className={styles.thumbnails}>
      {thumbnails.map((thumbnail, index) => (
        <li key={thumbnail.id} className={styles.thumbnail} style={indexStyle(index)}>
          <svg viewBox="0 0 96 72" aria-hidden="true">
            {thumbnail.paths.map((path) => (
              <path key={path} d={path} />
            ))}
          </svg>
          <span>{thumbnail.label}</span>
        </li>
      ))}
    </ul>
  );
}

/** A box whose lid opens on hover. */
function BoxLid() {
  return (
    <svg viewBox="0 0 120 96" aria-hidden="true" className={styles.boxArt}>
      <path className={styles.boxBody} d="M16 44 60 62 104 44V78L60 94 16 78Z" />
      <path className={styles.boxCrease} d="M60 62V94" />
      <path className={styles.lid} d="M16 44 60 26 104 44 60 62Z" />
    </svg>
  );
}

/** Fit-check legend: the three clearance colours (CLAUDE.md §7.3). */
function FitLegend() {
  const items = [
    { id: "ok", label: t("features.fit.ok") },
    { id: "tight", label: t("features.fit.tight") },
    { id: "collide", label: t("features.fit.collide") },
  ] as const;
  return (
    <ul className={styles.legend}>
      {items.map((item) => (
        <li key={item.id} className={`${styles.legendItem} ${styles[item.id]}`}>
          {item.label}
        </li>
      ))}
    </ul>
  );
}

/** Four A4 tiles with crop marks that slide together on hover. */
function PrintTiles() {
  const tiles = ["M6 6h72v50H6Z", "M82 6h72v50H82Z", "M6 60h72v50H6Z", "M82 60h72v50H82Z"];
  return (
    <svg viewBox="0 0 160 116" aria-hidden="true" className={styles.tiles}>
      {tiles.map((tile, index) => (
        <path key={tile} className={styles.sheetTile} d={tile} style={indexStyle(index)} />
      ))}
      <path className={styles.tiledOutline} d="M30 30h100v56H30Z" />
    </svg>
  );
}

/** Production file formats as pills. */
function FormatPills() {
  return (
    <ul className={styles.pills}>
      {EXPORT_FORMATS.map((format, index) => (
        <li key={format} className={styles.pill} style={indexStyle(index)}>
          {format}
        </li>
      ))}
    </ul>
  );
}

interface Tile {
  readonly id: "inserts" | "box" | "fit" | "test" | "files" | "checks";
  readonly icon: IconName;
  readonly title: MessageKey;
  readonly body: MessageKey;
  readonly extra: ReactNode;
}

/**
 * "What you get" as a bento grid of packaging-swatch tiles. Each tile lifts, glows under the
 * pointer and plays a small animation of its own on hover.
 */
export function FeatureBento() {
  const tiles: readonly Tile[] = [
    {
      id: "inserts",
      icon: "insert",
      title: "features.inserts.title",
      body: "features.inserts.body",
      extra: <InsertStyles />,
    },
    {
      id: "box",
      icon: "cube",
      title: "features.box.title",
      body: "features.box.body",
      extra: <BoxLid />,
    },
    {
      id: "fit",
      icon: "check",
      title: "features.fit.title",
      body: "features.fit.body",
      extra: <FitLegend />,
    },
    {
      id: "test",
      icon: "printer",
      title: "features.test.title",
      body: "features.test.body",
      extra: <PrintTiles />,
    },
    {
      id: "files",
      icon: "file",
      title: "features.files.title",
      body: "features.files.body",
      extra: <FormatPills />,
    },
    {
      id: "checks",
      icon: "shield",
      title: "features.checks.title",
      body: "features.checks.body",
      extra: <Icon name="shield" size={64} className={styles.shield} />,
    },
  ];

  return (
    <ul className={styles.grid}>
      {tiles.map((tile, index) => (
        <li
          key={tile.id}
          className={`${styles.cell} ${styles[tile.id]}`}
          data-reveal
          style={revealDelay(index)}
        >
          <article className={styles.tile} aria-labelledby={`feature-${tile.id}`} data-glow>
            <header className={styles.header}>
              <span className={styles.icon} aria-hidden="true">
                <Icon name={tile.icon} size={22} />
              </span>
              <h3 id={`feature-${tile.id}`} className={styles.title}>
                {t(tile.title)}
              </h3>
            </header>
            <p className={styles.body}>{t(tile.body)}</p>
            <figure className={styles.extra}>{tile.extra}</figure>
          </article>
        </li>
      ))}
    </ul>
  );
}
