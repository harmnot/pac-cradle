"use client";

import { startTransition, useState, type FormEvent } from "react";
import { StepTransition, TransitionType } from "@/components/motion/PageTransition";
import { Button, ButtonSize, ButtonVariant, LinkButton } from "@/components/ui/Button";
import { TextField } from "@/components/ui/TextField";
import { NEW_DESIGN_FROM_TYPED } from "@/features/design/routes";
import { AppRoute } from "@/features/identity/routes";
import { t } from "@/lib/i18n";
import { none, some, type Maybe } from "@/lib/maybe";
import {
  keepTypedProduct,
  TYPED_MAX_MM,
  TYPED_MIN_MM,
  TypedShape,
  validSize,
  type TypedProduct,
} from "../typed-product";
import { formatMm, sizeLabel } from "../format";
import styles from "./TypedStep.module.css";

const STEP_MM = 0.1;
/** The drawing's box (viewBox units) and its margin. */
const DRAWING = 120;
const MARGIN = 14;

/** The product from the form, when every size is a valid number. */
function readProduct({ shape, values }: { shape: TypedShape; values: Readonly<Record<string, string>> }): Maybe<TypedProduct> {
  const number = (name: string) => Number.parseFloat(values[name] ?? "");
  const height = number("height");
  if (shape === TypedShape.Round) {
    const diameter = number("diameter");
    return validSize(diameter) && validSize(height)
      ? some({ shape, diameterMm: diameter, heightMm: height })
      : none;
  }
  const length = number("length");
  const width = number("width");
  return validSize(length) && validSize(width) && validSize(height)
    ? some({ shape, lengthMm: Math.max(length, width), widthMm: Math.min(length, width), heightMm: height })
    : none;
}

/** The product seen from above, to scale, with its sizes. */
function TopView({ product }: { readonly product: TypedProduct }) {
  const across = product.shape === "round" ? product.diameterMm : product.lengthMm;
  const deep = product.shape === "round" ? product.diameterMm : product.widthMm;
  const scale = (DRAWING - 2 * MARGIN) / Math.max(across, deep);
  const w = across * scale;
  const h = deep * scale;
  const x = (DRAWING - w) / 2;
  const y = (DRAWING - h) / 2;
  return (
    <svg viewBox={`0 0 ${DRAWING} ${DRAWING}`} className={styles.topView} aria-hidden="true">
      {product.shape === "round" ? (
        <circle cx={DRAWING / 2} cy={DRAWING / 2} r={w / 2} pathLength={1} />
      ) : (
        <rect x={x} y={y} width={w} height={h} rx={3} pathLength={1} />
      )}
      <path className={styles.dim} d={`M${x} ${y - 6}H${x + w}`} pathLength={1} />
    </svg>
  );
}

/** A typed product's size from above. */
function sizeText(product: TypedProduct): string {
  return product.shape === "round"
    ? sizeLabel({ shape: "round", diameterMm: product.diameterMm })
    : sizeLabel({ shape: "box", lengthMm: product.lengthMm, widthMm: product.widthMm });
}

/**
 * Typing the sizes, with no camera (CLAUDE.md §3 step 1c): round or rectangular, the sizes
 * from a ruler, and the height. The product waits in this tab for the design choices (E4).
 */
export function TypedStep({ onBack }: { readonly onBack: () => void }) {
  const [shape, setShape] = useState<TypedShape>(TypedShape.Round);
  const [values, setValues] = useState<Readonly<Record<string, string>>>({});
  const [kept, setKept] = useState<Maybe<TypedProduct>>(none);

  const field = (name: string) => ({
    name,
    type: "number" as const,
    inputMode: "decimal" as const,
    required: true,
    min: TYPED_MIN_MM,
    max: TYPED_MAX_MM,
    step: STEP_MM,
    value: values[name] ?? "",
    onChange: (event: { target: HTMLInputElement }) =>
      setValues((current) => ({ ...current, [name]: event.target.value })),
  });

  const submit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const product = readProduct({ shape, values });
    if (product.kind === "some") {
      keepTypedProduct(product.value);
      startTransition(() => setKept(product));
    }
  };

  return (
    <StepTransition step={kept.kind === "some" ? "kept" : "form"}>
      {kept.kind === "some" ? (
        <section className={styles.step} aria-labelledby="typed-done-title">
          <header className={styles.head}>
            <h1 id="typed-done-title" className={styles.title}>
              {t("capture.typed.doneTitle")}
            </h1>
          </header>
          <div className={styles.summary}>
            <TopView product={kept.value} />
            <dl className={styles.facts}>
              <div>
                <dt>{t("capture.result.size")}</dt>
                <dd>{sizeText(kept.value)}</dd>
              </div>
              <div>
                <dt>{t("capture.result.height")}</dt>
                <dd>{t("capture.size.mm", { mm: formatMm(kept.value.heightMm) })}</dd>
              </div>
            </dl>
          </div>
          <div className={styles.actions}>
            <LinkButton
              href={NEW_DESIGN_FROM_TYPED}
              variant={ButtonVariant.Accent}
              size={ButtonSize.Large}
              withArrow
              transitionTypes={[TransitionType.Forward]}
            >
              {t("capture.next.action")}
            </LinkButton>
            <Button variant={ButtonVariant.Secondary} onClick={() => startTransition(() => setKept(none))}>
              {t("capture.typed.edit")}
            </Button>
            <LinkButton href={AppRoute.Workspace} variant={ButtonVariant.Quiet}>
              {t("capture.toWorkspace")}
            </LinkButton>
          </div>
        </section>
      ) : (
        <section className={styles.step} aria-labelledby="typed-title">
          <header className={styles.head}>
            <h1 id="typed-title" className={styles.title}>
              {t("capture.typed.title")}
            </h1>
            <p className={styles.lead}>{t("capture.typed.lead")}</p>
          </header>
          <form className={styles.form} onSubmit={submit}>
            <div role="radiogroup" aria-label={t("capture.typed.shape")} className={styles.shapes}>
              {[TypedShape.Round, TypedShape.Rectangular].map((option) => (
                <button
                  key={option}
                  type="button"
                  role="radio"
                  aria-checked={shape === option}
                  className={styles.shape}
                  onClick={() => setShape(option)}
                >
                  <span className={option === TypedShape.Round ? styles.round : styles.box} aria-hidden="true" />
                  {option === TypedShape.Round ? t("capture.typed.round") : t("capture.typed.rectangular")}
                </button>
              ))}
            </div>
            {shape === TypedShape.Round ? (
              <TextField label={t("capture.typed.diameter")} hint={t("capture.typed.mmHint")} {...field("diameter")} />
            ) : (
              <div className={styles.pair}>
                <TextField label={t("capture.typed.length")} {...field("length")} />
                <TextField label={t("capture.typed.width")} {...field("width")} />
              </div>
            )}
            <TextField label={t("capture.typed.height")} hint={t("capture.typed.heightHint")} {...field("height")} />
            <div className={styles.actions}>
              <Button type="submit" size={ButtonSize.Large} withArrow>
                {t("capture.typed.submit")}
              </Button>
              <Button variant={ButtonVariant.Quiet} onClick={onBack}>
                {t("capture.back")}
              </Button>
            </div>
          </form>
        </section>
      )}
    </StepTransition>
  );
}
