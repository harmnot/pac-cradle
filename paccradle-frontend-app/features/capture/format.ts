import { t } from "@/lib/i18n";

/** Sizes read to a tenth of a millimetre, like a calliper. */
const MM_FORMAT = new Intl.NumberFormat("en-US", { maximumFractionDigits: 1, minimumFractionDigits: 0 });

/** A length, e.g. "52.3". */
export function formatMm(value: number): string {
  return MM_FORMAT.format(value);
}

/** A product's size from above: "⌀ 52.3 mm" when round, else "80 × 40.5 mm". */
export function sizeLabel(
  size:
    | { readonly shape: "round"; readonly diameterMm: number }
    | { readonly shape: "box"; readonly lengthMm: number; readonly widthMm: number },
): string {
  return size.shape === "round"
    ? t("capture.size.round", { d: formatMm(size.diameterMm) })
    : t("capture.size.box", { l: formatMm(size.lengthMm), w: formatMm(size.widthMm) });
}
