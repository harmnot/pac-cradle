"use client";

import type { CSSProperties } from "react";
import { ButtonVariant, LinkButton } from "@/components/ui/Button";
import { ChoiceChips } from "@/components/ui/ChoiceChips";
import { Icon } from "@/components/ui/Icon";
import { Notice } from "@/components/ui/Notice";
import { NumberStepper } from "@/components/ui/NumberStepper";
import { TextField } from "@/components/ui/TextField";
import { formatMm, sizeLabel } from "@/features/capture/format";
import type { PhotoWarning } from "@/features/capture/types";
import { t, tCount, type MessageKey } from "@/lib/i18n";
import { none, some, type Maybe } from "@/lib/maybe";
import {
  HEIGHT_MAX_MM,
  HEIGHT_MIN_MM,
  ProductSource,
  QUANTITY_MAX,
  QUANTITY_MIN,
  QUANTITY_PRESETS,
  type ChosenProduct,
} from "../choices";
import { polygonPoints, boundsOf } from "../preview";
import styles from "./ProductChoice.module.css";

/** The outline drawing's box (viewBox units) and its margin. */
const OUTLINE_BOX = 120;
const OUTLINE_MARGIN = 16;
/** The side view's tallest product, in mm: taller ones are drawn at this height. */
const SIDE_VIEW_MAX_MM = 300;
const HEIGHT_STEP_MM = 0.1;

/** Where the product came from, in a few words. */
const SOURCE_LABEL: Readonly<Record<ProductSource, MessageKey>> = {
  [ProductSource.Measured]: "choices.product.measured",
  [ProductSource.Drawn]: "choices.product.drawn",
  [ProductSource.Typed]: "choices.product.typed",
};

/** Sending the height to the capture (to recheck the tall-product warning). */
export type HeightCheck =
  | { readonly kind: "idle" }
  | { readonly kind: "sending" }
  | { readonly kind: "failed"; readonly message: string };

/** Props for the product section. */
export interface ProductChoiceProps {
  readonly product: ChosenProduct;
  readonly heightText: string;
  readonly heightValid: boolean;
  readonly onHeight: (text: string) => void;
  readonly quantity: number;
  readonly onQuantity: (quantity: number) => void;
  /** The capture's warnings about the photo (none for typed sizes). */
  readonly warning: Maybe<PhotoWarning>;
  readonly heightCheck: HeightCheck;
  /** Back to where the sizes came from (the capture, or the typed form). */
  readonly changeHref: string;
  /** Offer Trace mode next to a parallax warning (photo captures only). */
  readonly traceHref: Maybe<string>;
}

/** The product from above, to scale, with its outline drawn in. */
function OutlineThumb({ product }: { readonly product: ChosenProduct }) {
  const box = boundsOf(product.contourMm);
  const scale = (OUTLINE_BOX - 2 * OUTLINE_MARGIN) / Math.max(box.width, box.height, 1);
  const shifted = product.contourMm.map(
    (point) =>
      [
        OUTLINE_MARGIN + (point[0] - box.x) * scale + (OUTLINE_BOX - 2 * OUTLINE_MARGIN - box.width * scale) / 2,
        OUTLINE_MARGIN + (point[1] - box.y) * scale + (OUTLINE_BOX - 2 * OUTLINE_MARGIN - box.height * scale) / 2,
      ] as const,
  );
  return (
    <svg viewBox={`0 0 ${OUTLINE_BOX} ${OUTLINE_BOX}`} className={styles.thumb} aria-hidden="true">
      <polygon className={styles.thumbFill} points={polygonPoints(shifted)} />
      <polygon className={styles.thumbLine} points={polygonPoints(shifted)} pathLength={1} />
    </svg>
  );
}

/** A side view whose height follows the field: the product grows against a ruler. */
function SideView({ product, heightMm }: { readonly product: ChosenProduct; readonly heightMm: Maybe<number> }) {
  const share = heightMm.kind === "some" ? Math.min(heightMm.value, SIDE_VIEW_MAX_MM) / SIDE_VIEW_MAX_MM : 0;
  const widthShare = Math.min(product.lengthMm, SIDE_VIEW_MAX_MM) / SIDE_VIEW_MAX_MM;
  return (
    <span
      className={styles.side}
      data-known={heightMm.kind === "some"}
      style={{ "--height": share, "--width": widthShare } as CSSProperties}
      aria-hidden="true"
    >
      <span className={styles.ruler} />
      <span className={styles.body} data-round={product.round} />
    </span>
  );
}

/** The warning's words, with the numbers filled in. */
function warningText(warning: PhotoWarning): string {
  switch (warning.kind) {
    case "outline_may_be_bigger":
      return t("warning.outline_may_be_bigger", {
        up_to_mm: formatMm(warning.up_to_mm),
        height_mm: formatMm(warning.height_mm),
      });
    case "other_objects":
      return tCount({ base: "warning.other_objects", count: warning.count });
    default:
      return t(`warning.${warning.kind}`);
  }
}

/**
 * Section 1: the product as measured or typed, its height (§3 step 5) and how many go in
 * one box (step 6). A photo's tall-product warning sits under the height with a way to
 * Trace mode; the sizes are never rounded down by it (it's a bound, M3-03 corrects).
 */
export function ProductChoice({
  product,
  heightText,
  heightValid,
  onHeight,
  quantity,
  onQuantity,
  warning,
  heightCheck,
  changeHref,
  traceHref,
}: ProductChoiceProps) {
  const size = product.round
    ? sizeLabel({ shape: "round", diameterMm: product.lengthMm })
    : sizeLabel({ shape: "box", lengthMm: product.lengthMm, widthMm: product.widthMm });
  const heightMm = heightValid ? some(Number.parseFloat(heightText)) : none;
  const showError = heightText.trim() !== "" && !heightValid;
  return (
    <div className={styles.product}>
      <div className={styles.card}>
        <OutlineThumb product={product} />
        <div className={styles.facts}>
          <p className={styles.size}>{size}</p>
          <p className={styles.source}>
            <Icon name={product.source === ProductSource.Typed ? "ruler" : "camera"} size={16} />
            {t(SOURCE_LABEL[product.source])}
          </p>
          <LinkButton href={changeHref} variant={ButtonVariant.Quiet}>
            {t("choices.product.change")}
          </LinkButton>
        </div>
      </div>

      <div className={styles.height}>
        <TextField
          label={t("choices.height.label")}
          hint={t("choices.height.hint")}
          error={showError ? t("choices.height.invalid", { min: HEIGHT_MIN_MM, max: HEIGHT_MAX_MM }) : undefined}
          type="number"
          inputMode="decimal"
          min={HEIGHT_MIN_MM}
          max={HEIGHT_MAX_MM}
          step={HEIGHT_STEP_MM}
          required
          value={heightText}
          placeholder={t("choices.height.placeholder")}
          onChange={(event) => onHeight(event.target.value)}
        />
        <SideView product={product} heightMm={heightMm} />
      </div>
      {heightCheck.kind === "sending" && (
        <p className={styles.checking} role="status">
          <span className={styles.dot} aria-hidden="true" />
          {t("choices.height.checking")}
        </p>
      )}
      {heightCheck.kind === "failed" && <p className={styles.checkFailed}>{heightCheck.message}</p>}
      {warning.kind === "some" && (
        <Notice
          action={
            traceHref.kind === "some" && (
              <LinkButton href={traceHref.value} variant={ButtonVariant.Secondary}>
                {t("capture.action.trace")}
              </LinkButton>
            )
          }
        >
          {warningText(warning.value)}
        </Notice>
      )}

      <div className={styles.quantity}>
        <NumberStepper
          label={t("choices.quantity.label")}
          value={quantity}
          min={QUANTITY_MIN}
          max={QUANTITY_MAX}
          onChange={onQuantity}
          decreaseLabel={t("choices.quantity.fewer")}
          increaseLabel={t("choices.quantity.more")}
          unit={t("choices.quantity.unit")}
        />
        <ChoiceChips<number>
          label={t("choices.quantity.presets")}
          options={QUANTITY_PRESETS.map((count) => ({ value: count, label: String(count) }))}
          value={quantity}
          onChange={onQuantity}
        />
      </div>
    </div>
  );
}
