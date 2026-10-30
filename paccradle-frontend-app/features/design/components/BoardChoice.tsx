"use client";

import { useState } from "react";
import { ChoiceCards } from "@/components/ui/ChoiceCards";
import { TextField } from "@/components/ui/TextField";
import { formatMm } from "@/features/capture/format";
import { t } from "@/lib/i18n";
import { BOARDS, CLEARANCE_RANGE_MM, THICKNESS_RANGE_MM, TYPICAL_THICKNESS_MM } from "../choices";
import type { TuningField } from "../design-choices-state";
import { STARTING_MATERIALS } from "../samples";
import type { Board, Material } from "../types";
import { BoardSection } from "./ChoiceArt";
import styles from "./BoardChoice.module.css";

const TUNING_STEP_MM = 0.1;

/** Props for the board section. */
export interface BoardChoiceProps {
  readonly labelledBy: string;
  readonly board: Board;
  readonly recommended: Board;
  readonly onBoard: (board: Board) => void;
  /** The board as the design uses it (starting values or the person's). */
  readonly material: Material;
  readonly thicknessText: string;
  readonly thickness: TuningField;
  readonly onThickness: (text: string) => void;
  readonly clearanceText: string;
  readonly clearance: TuningField;
  readonly onClearance: (text: string) => void;
}

/**
 * Section 3 (§3 step 8, §6.4): the insert's board, each card showing its edge cut through to
 * scale, the usual one for the style tagged. The printer's own thickness and the clearance
 * round each product can be set under "Fine-tune".
 */
export function BoardChoice({
  labelledBy,
  board,
  recommended,
  onBoard,
  material,
  thicknessText,
  thickness,
  onThickness,
  clearanceText,
  clearance,
  onClearance,
}: BoardChoiceProps) {
  const typical = TYPICAL_THICKNESS_MM[board];
  const start = STARTING_MATERIALS[board];
  // Open while the person has it open; it starts open when the board is already tuned.
  const [open, setOpen] = useState(thickness.kind !== "start" || clearance.kind !== "start");
  return (
    <div className={styles.board}>
      <ChoiceCards
        labelledBy={labelledBy}
        size="compact"
        value={board}
        onChange={onBoard}
        options={BOARDS.map((option) => ({
          value: option,
          title: t(`board.${option}`),
          meta: t("choices.board.thickness", { mm: formatMm(STARTING_MATERIALS[option].thickness_mm) }),
          body: t(`board.${option}.use`),
          badge: option === recommended ? t("choices.recommended") : undefined,
          art: <BoardSection board={option} thicknessMm={STARTING_MATERIALS[option].thickness_mm} />,
        }))}
      />
      <details className={styles.tune} open={open} onToggle={(event) => setOpen(event.currentTarget.open)}>
        <summary className={styles.summary}>
          <span>{t("choices.board.tune")}</span>
          <span className={styles.current}>
            {t("choices.board.current", {
              thickness: formatMm(material.thickness_mm),
              clearance: formatMm(material.clearance_mm),
            })}
          </span>
        </summary>
        <div className={styles.fields}>
          <TextField
            label={t("choices.board.thicknessLabel")}
            hint={t("choices.board.thicknessHint", { min: formatMm(typical.min), max: formatMm(typical.max) })}
            error={thickness.kind === "invalid" ? t("choices.board.thicknessInvalid", { max: THICKNESS_RANGE_MM.max }) : undefined}
            type="number"
            inputMode="decimal"
            min={THICKNESS_RANGE_MM.min}
            max={THICKNESS_RANGE_MM.max}
            step={TUNING_STEP_MM}
            placeholder={formatMm(start.thickness_mm)}
            value={thicknessText}
            onChange={(event) => onThickness(event.target.value)}
          />
          <TextField
            label={t("choices.board.clearanceLabel")}
            hint={t("choices.board.clearanceHint")}
            error={clearance.kind === "invalid" ? t("choices.board.clearanceInvalid", { max: CLEARANCE_RANGE_MM.max }) : undefined}
            type="number"
            inputMode="decimal"
            min={CLEARANCE_RANGE_MM.min}
            max={CLEARANCE_RANGE_MM.max}
            step={TUNING_STEP_MM}
            placeholder={formatMm(start.clearance_mm)}
            value={clearanceText}
            onChange={(event) => onClearance(event.target.value)}
          />
        </div>
        <p className={styles.note}>{t("choices.board.printer")}</p>
      </details>
    </div>
  );
}
