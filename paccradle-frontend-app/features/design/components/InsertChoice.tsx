import { ChoiceCards } from "@/components/ui/ChoiceCards";
import { ChoiceChips } from "@/components/ui/ChoiceChips";
import { Icon } from "@/components/ui/Icon";
import { t, tCount, type MessageKey } from "@/lib/i18n";
import type { Maybe } from "@/lib/maybe";
import { gridLayouts, sameLayout, type GridLayout, type StyleReason } from "../choices";
import { InsertStyle } from "../samples";
import { ChoiceArt } from "./ChoiceArt";
import styles from "./InsertChoice.module.css";

/** Each style's name and what it suits (CLAUDE.md §6.2), in the order offered. */
const STYLES: ReadonlyArray<{ readonly style: InsertStyle; readonly title: MessageKey; readonly body: MessageKey }> = [
  { style: InsertStyle.Grid, title: "style.partition_grid", body: "choices.style.partition_grid.body" },
  { style: InsertStyle.Card, title: "style.cutout_card", body: "choices.style.cutout_card.body" },
  { style: InsertStyle.Cradle, title: "style.cradle_platform", body: "choices.style.cradle_platform.body" },
];

/** A grid layout's cells, the filled ones sage: a tiny plan of the grid. */
function LayoutArt({ layout, quantity }: { readonly layout: GridLayout; readonly quantity: number }) {
  const CELL = 6;
  const GAP = 1.5;
  const cells = Array.from({ length: layout.rows * layout.columns }, (_, index) => index);
  return (
    <svg
      className={styles.layoutArt}
      viewBox={`0 0 ${layout.columns * (CELL + GAP) - GAP} ${layout.rows * (CELL + GAP) - GAP}`}
      aria-hidden="true"
    >
      {cells.map((index) => (
        <rect
          key={index}
          x={(index % layout.columns) * (CELL + GAP)}
          y={Math.floor(index / layout.columns) * (CELL + GAP)}
          width={CELL}
          height={CELL}
          rx={1.2}
          data-filled={index < quantity}
        />
      ))}
    </svg>
  );
}

/** Props for the insert section. */
export interface InsertChoiceProps {
  readonly labelledBy: string;
  readonly style: InsertStyle;
  readonly onStyle: (style: InsertStyle) => void;
  readonly recommendation: Maybe<{ readonly style: InsertStyle; readonly reason: StyleReason }>;
  readonly grid: GridLayout;
  readonly onGrid: (grid: GridLayout) => void;
  readonly quantity: number;
}

/**
 * Section 2 (§3 step 7): how the insert holds the product, as cards with a drawing each, the
 * suggested one tagged with why; a grid also asks for its layout.
 */
export function InsertChoice({ labelledBy, style, onStyle, recommendation, grid, onGrid, quantity }: InsertChoiceProps) {
  const recommended = recommendation.kind === "some" ? recommendation.value.style : undefined;
  const layouts = gridLayouts(quantity);
  const empty = grid.rows * grid.columns - quantity;
  const key = (layout: GridLayout) => `${layout.rows}x${layout.columns}`;
  return (
    <div className={styles.insert}>
      <ChoiceCards
        labelledBy={labelledBy}
        value={style}
        onChange={onStyle}
        options={STYLES.map((option) => ({
          value: option.style,
          title: t(option.title),
          body: t(option.body),
          badge: option.style === recommended ? t("choices.recommended") : undefined,
          art: <ChoiceArt name={option.style} />,
        }))}
      />
      {recommendation.kind === "some" && (
        <p className={styles.why}>
          <Icon name="sparkle" size={16} />
          <span>
            <strong>{t(`style.${recommendation.value.style}`)}</strong>{" "}
            {t(`choices.style.why.${recommendation.value.reason}`)}
          </span>
        </p>
      )}
      {style === InsertStyle.Grid && (
        <div className={styles.layouts} key={quantity}>
          <p className={styles.layoutsTitle}>{t("choices.grid.title")}</p>
          <ChoiceChips<string>
            label={t("choices.grid.title")}
            value={key(grid)}
            onChange={(value) => {
              const chosen = layouts.find((layout) => key(layout) === value);
              if (chosen !== undefined && !sameLayout(chosen, grid)) {
                onGrid(chosen);
              }
            }}
            options={layouts.map((layout) => ({
              value: key(layout),
              label: t("choices.grid.layout", { rows: layout.rows, columns: layout.columns }),
              art: <LayoutArt layout={layout} quantity={quantity} />,
            }))}
          />
          {empty > 0 && <p className={styles.note}>{tCount({ base: "choices.grid.empty", count: empty })}</p>}
        </div>
      )}
    </div>
  );
}
