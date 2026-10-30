import { ChoiceCards } from "@/components/ui/ChoiceCards";
import { ChoiceChips } from "@/components/ui/ChoiceChips";
import { t, type MessageKey } from "@/lib/i18n";
import { BOARDS, BoxChoice } from "../choices";
import type { Board } from "../types";
import { ChoiceArt } from "./ChoiceArt";
import styles from "./OuterBoxChoice.module.css";

/** The box styles (CLAUDE.md §6.3), then the insert on its own. */
const BOXES: ReadonlyArray<{ readonly box: BoxChoice; readonly title: MessageKey; readonly body: MessageKey; readonly meta?: MessageKey }> = [
  { box: BoxChoice.Mailer, title: "choices.box.mailer", body: "choices.box.mailer.body", meta: "choices.box.mailer.code" },
  { box: BoxChoice.Shipping, title: "choices.box.shipping", body: "choices.box.shipping.body", meta: "choices.box.shipping.code" },
  { box: BoxChoice.None, title: "choices.box.none", body: "choices.box.none.body" },
];

/** Props for the box section. */
export interface OuterBoxChoiceProps {
  readonly labelledBy: string;
  readonly box: BoxChoice;
  readonly onBox: (box: BoxChoice) => void;
  readonly boxBoard: Board;
  readonly onBoxBoard: (board: Board) => void;
}

/**
 * Section 4 (§3 step 8, U4): the box sized round the insert (a mailer or a shipping box), or
 * none, and the box's own board. Its sizes and empty space show in the preview.
 */
export function OuterBoxChoice({ labelledBy, box, onBox, boxBoard, onBoxBoard }: OuterBoxChoiceProps) {
  return (
    <div className={styles.box}>
      <ChoiceCards
        labelledBy={labelledBy}
        value={box}
        onChange={onBox}
        options={BOXES.map((option) => ({
          value: option.box,
          title: t(option.title),
          body: t(option.body),
          meta: option.meta === undefined ? undefined : t(option.meta),
          art: <ChoiceArt name={option.box} />,
        }))}
      />
      {box !== BoxChoice.None && (
        <div className={styles.board} key={box}>
          <p className={styles.title}>{t("choices.box.board")}</p>
          <ChoiceChips<Board>
            label={t("choices.box.board")}
            value={boxBoard}
            onChange={onBoxBoard}
            options={BOARDS.map((board) => ({ value: board, label: t(`board.${board}`) }))}
          />
        </div>
      )}
    </div>
  );
}
