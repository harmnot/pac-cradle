"use client";

import { useRouter, useSearchParams } from "next/navigation";
import {
  startTransition,
  useEffect,
  useId,
  useMemo,
  useReducer,
  useRef,
  useState,
  type CSSProperties,
  type FormEvent,
  type ReactNode,
} from "react";
import { TransitionType } from "@/components/motion/PageTransition";
import { Button, ButtonSize, ButtonVariant, LinkButton } from "@/components/ui/Button";
import { Icon } from "@/components/ui/Icon";
import { Notice, NoticeTone } from "@/components/ui/Notice";
import { Skeleton, SkeletonShape } from "@/components/ui/Skeleton";
import { TextField } from "@/components/ui/TextField";
import { correctCapture } from "@/features/capture/api";
import { sizeLabel } from "@/features/capture/format";
import { useKeptTypedProduct } from "@/features/capture/typed-product";
import { CaptureMode, CaptureStatus, type Capture, type PhotoWarning } from "@/features/capture/types";
import { useCapture } from "@/features/capture/use-capture";
import { AppRoute } from "@/features/identity/routes";
import { useSession } from "@/features/identity/session";
import { describeError } from "@/lib/errors";
import { t } from "@/lib/i18n";
import { none, some, type Maybe } from "@/lib/maybe";
import { createDesign, DESIGN_NAME_MAX } from "../api";
import { buildDesign, measuredProduct, typedProduct, type ChosenProduct } from "../choices";
import { initialChoices, reduceChoices, resolveChoices, type ChoicesAction } from "../design-choices-state";
import { runPreview } from "../preview";
import { CAPTURE_PARAM, designPath } from "../routes";
import { useInsertCore } from "../use-insert-core";
import { BoardChoice } from "./BoardChoice";
import { InsertChoice } from "./InsertChoice";
import { OuterBoxChoice } from "./OuterBoxChoice";
import { PackagingPreview } from "./PackagingPreview";
import { ProductChoice, type HeightCheck } from "./ProductChoice";
import styles from "./DesignChoices.module.css";

/** Wait this long after the last keystroke before sending the height to the capture. */
const HEIGHT_SEND_DELAY_MS = 700;
/** Heights this close count as the same (the field holds tenths). */
const SAME_HEIGHT_MM = 0.05;
/** Codes that mean this capture can't be read from here. */
const NOT_HERE = new Set(["CAPTURE_NOT_FOUND", "AUTH_SESSION_EXPIRED", "INVALID_REQUEST"]);

/** A capture the design starts from, and how to show a correction's answer. */
interface CaptureLink {
  readonly capture: Capture;
  readonly apply: (capture: Capture) => void;
}

/** Saving the design. */
type SaveState = { readonly kind: "idle" } | { readonly kind: "saving" } | { readonly kind: "failed"; readonly message: string };

/** One numbered section of the choices. */
function Section({
  index,
  headingId,
  title,
  lead,
  tone = "paper",
  children,
}: {
  readonly index: number;
  readonly headingId: string;
  readonly title: string;
  readonly lead?: string;
  readonly tone?: "paper" | "forest";
  readonly children: ReactNode;
}) {
  return (
    <section
      className={styles.section}
      data-tone={tone}
      data-step={index}
      aria-labelledby={headingId}
      style={{ "--index": index } as CSSProperties}
    >
      <header className={styles.sectionHead}>
        <span className={styles.number} aria-hidden="true">
          {index}
        </span>
        <div className={styles.sectionText}>
          <h2 id={headingId} className={styles.sectionTitle}>
            {title}
          </h2>
          {lead !== undefined && <p className={styles.sectionLead}>{lead}</p>}
        </div>
      </header>
      {children}
    </section>
  );
}

/** The capture's warning about the product's height: the bound for this height, else "tall products look bigger". */
function heightWarning({ warnings, heightMm }: { warnings: readonly PhotoWarning[]; heightMm: Maybe<number> }): Maybe<PhotoWarning> {
  const bound = warnings.find(
    (warning) =>
      warning.kind === "outline_may_be_bigger" &&
      heightMm.kind === "some" &&
      Math.abs(warning.height_mm - heightMm.value) < SAME_HEIGHT_MM,
  );
  if (bound !== undefined) {
    return some(bound);
  }
  const tall = warnings.find((warning) => warning.kind === "tall_product_looks_bigger");
  return tall !== undefined && heightMm.kind === "none" ? some(tall) : none;
}

/**
 * Sends a typed height to the capture (`correct {height_mm}`, M1-C4/D5) once typing settles,
 * so vision rechecks how far a tall product's outline may reach. Only the latest answer is shown.
 */
function useHeightCheck({ link, heightMm, storedMm }: { link: Maybe<CaptureLink>; heightMm: Maybe<number>; storedMm: Maybe<number> }): HeightCheck {
  const [check, setCheck] = useState<HeightCheck>({ kind: "idle" });
  const latest = useRef(0);
  const captureId = link.kind === "some" ? some(link.value.capture.id) : none;
  const apply = link.kind === "some" ? some(link.value.apply) : none;
  const applyRef = useRef(apply);

  useEffect(() => {
    applyRef.current = apply;
  });

  const id = captureId.kind === "some" ? captureId.value : "";
  const height = heightMm.kind === "some" ? heightMm.value : Number.NaN;
  const stored = storedMm.kind === "some" ? storedMm.value : Number.NaN;

  useEffect(() => {
    if (id === "" || Number.isNaN(height) || Math.abs(height - stored) < SAME_HEIGHT_MM) {
      return;
    }
    const timer = setTimeout(() => {
      const send = async () => {
        latest.current += 1;
        const mine = latest.current;
        setCheck({ kind: "sending" });
        const result = await correctCapture({ id, correction: { kind: "height", mm: height } });
        if (mine !== latest.current) {
          return;
        }
        if (result.kind === "error") {
          setCheck({ kind: "failed", message: describeError(result.error) });
          return;
        }
        setCheck({ kind: "idle" });
        if (applyRef.current.kind === "some") {
          applyRef.current.value(result.value);
        }
      };
      void send();
    }, HEIGHT_SEND_DELAY_MS);
    return () => clearTimeout(timer);
  }, [id, height, stored]);

  return check;
}

/** Props for the choices form. */
interface ChoicesFormProps {
  readonly product: ChosenProduct;
  readonly link: Maybe<CaptureLink>;
  /** Back to where the sizes came from. */
  readonly changeHref: string;
}

/**
 * The design choices (CLAUDE.md §3 steps 5–8): the product's height and count, the insert
 * style, its board and the outer box, beside a live preview of what they make. Saving creates
 * the design (`POST /designs`, a guest session first) and opens it.
 */
function ChoicesForm({ product, link, changeHref }: ChoicesFormProps) {
  const router = useRouter();
  const { ensureSession } = useSession();
  const [state, dispatch] = useReducer(reduceChoices, product, initialChoices);
  const [save, setSave] = useState<SaveState>({ kind: "idle" });
  const { engine, retry } = useInsertCore();
  const ids = { product: useId(), insert: useId(), board: useId(), box: useId(), save: useId() };

  const resolved = useMemo(() => resolveChoices(state), [state]);
  const design = useMemo(
    () => (resolved.choices.kind === "some" ? some(buildDesign({ product, choices: resolved.choices.value })) : none),
    [product, resolved.choices],
  );
  const frame = useMemo(
    () => (engine.kind === "ready" && design.kind === "some" ? some(runPreview({ core: engine.core, design: design.value })) : none),
    [engine, design],
  );

  const capture = link.kind === "some" ? some(link.value.capture) : none;
  const heightCheck = useHeightCheck({ link, heightMm: resolved.heightMm, storedMm: product.heightMm });
  const warning =
    capture.kind === "some" && capture.value.result.kind === "some"
      ? heightWarning({ warnings: capture.value.result.value.warnings, heightMm: resolved.heightMm })
      : none;
  const traceHref =
    capture.kind === "some" && capture.value.mode === CaptureMode.Photo ? some(`${AppRoute.Capture}?mode=trace`) : none;

  /** A pick, made in a transition so the preview morphs to it. */
  const choose = (action: ChoicesAction) => startTransition(() => dispatch(action));

  const size = product.round
    ? sizeLabel({ shape: "round", diameterMm: product.lengthMm })
    : sizeLabel({ shape: "box", lengthMm: product.lengthMm, widthMm: product.widthMm });
  const suggestedName = t("choices.name.suggested", {
    style: t(`style.${resolved.style}`),
    count: state.quantity,
    size,
  });
  const name = state.name.kind === "some" ? state.name.value : suggestedName;
  const broken = frame.kind === "some" && frame.value.kind === "error";
  const canSave = design.kind === "some" && !broken && name.trim().length <= DESIGN_NAME_MAX;

  const create = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (design.kind === "none" || !canSave) {
      return;
    }
    setSave({ kind: "saving" });
    const session = await ensureSession();
    if (session.kind === "error") {
      setSave({ kind: "failed", message: describeError(session.error) });
      return;
    }
    const result = await createDesign({ name: name.trim(), spec: design.value });
    if (result.kind === "error") {
      setSave({ kind: "failed", message: describeError(result.error) });
      return;
    }
    router.push(designPath(result.value.id), { transitionTypes: [TransitionType.Forward] });
  };

  const saveHint = () => {
    if (design.kind === "none") {
      return t("choices.save.needHeight");
    }
    return broken ? t("choices.save.cantYet") : t("choices.save.free");
  };

  return (
    <div className={styles.page}>
      <LinkButton href={changeHref} variant={ButtonVariant.Quiet}>
        {t(link.kind === "some" ? "choices.back.capture" : "choices.back.typed")}
      </LinkButton>
      <header className={styles.head}>
        <p className={styles.eyebrow}>{t("choices.eyebrow")}</p>
        <h1 className={styles.title}>{t("choices.title")}</h1>
        <p className={styles.lead}>{t("choices.lead")}</p>
      </header>

      <div className={styles.layout}>
        <div className={styles.sections}>
          <Section index={1} headingId={ids.product} title={t("choices.section.product")} lead={t("choices.section.product.lead")}>
            <ProductChoice
              product={product}
              heightText={state.heightText}
              heightValid={resolved.heightMm.kind === "some"}
              onHeight={(text) => dispatch({ type: "height", text })}
              quantity={state.quantity}
              onQuantity={(quantity) => choose({ type: "quantity", quantity })}
              warning={warning}
              heightCheck={heightCheck}
              changeHref={changeHref}
              traceHref={traceHref}
            />
          </Section>

          <aside className={styles.aside} aria-label={t("preview.title")}>
            <PackagingPreview
              engine={engine}
              onRetry={retry}
              design={design}
              frame={frame}
              waiting={t("choices.preview.waiting")}
            />
          </aside>

          <Section index={2} headingId={ids.insert} title={t("choices.section.insert")} lead={t("choices.section.insert.lead")}>
            <InsertChoice
              labelledBy={ids.insert}
              style={resolved.style}
              onStyle={(style) => choose({ type: "style", style })}
              recommendation={resolved.recommendation}
              grid={resolved.grid}
              onGrid={(grid) => choose({ type: "grid", grid })}
              quantity={state.quantity}
            />
          </Section>

          <Section index={3} headingId={ids.board} title={t("choices.section.board")} lead={t("choices.section.board.lead")}>
            <BoardChoice
              labelledBy={ids.board}
              board={resolved.board}
              recommended={resolved.recommendedBoard}
              onBoard={(board) => choose({ type: "board", board })}
              material={resolved.material}
              thicknessText={state.thicknessText}
              thickness={resolved.thickness}
              onThickness={(text) => dispatch({ type: "thickness", text })}
              clearanceText={state.clearanceText}
              clearance={resolved.clearance}
              onClearance={(text) => dispatch({ type: "clearance", text })}
            />
          </Section>

          <Section index={4} headingId={ids.box} title={t("choices.section.box")} lead={t("choices.section.box.lead")}>
            <OuterBoxChoice
              labelledBy={ids.box}
              box={state.box}
              onBox={(box) => choose({ type: "box", box })}
              boxBoard={resolved.boxBoard}
              onBoxBoard={(board) => choose({ type: "boxBoard", board })}
            />
          </Section>

          <Section index={5} headingId={ids.save} title={t("choices.section.save")} tone="forest">
            <form className={styles.save} onSubmit={(event) => void create(event)}>
              <TextField
                label={t("choices.name.label")}
                maxLength={DESIGN_NAME_MAX}
                value={name}
                onChange={(event) => dispatch({ type: "name", name: event.target.value })}
              />
              <div className={styles.saveActions}>
                <Button
                  type="submit"
                  variant={ButtonVariant.Accent}
                  size={ButtonSize.Large}
                  withArrow
                  busy={save.kind === "saving"}
                  disabled={!canSave}
                >
                  {t("choices.save.action")}
                </Button>
                <p className={styles.saveHint}>
                  <Icon name={design.kind === "some" && !broken ? "shield" : "alert"} size={16} />
                  {saveHint()}
                </p>
              </div>
              {save.kind === "failed" && <Notice tone={NoticeTone.Error}>{save.message}</Notice>}
            </form>
          </Section>
        </div>
      </div>
    </div>
  );
}

/** A page with nothing to design from: why, and the ways on. */
function NothingHere({ title, lead, children }: { readonly title: string; readonly lead: string; readonly children: ReactNode }) {
  return (
    <div className={styles.missing}>
      <span className={styles.missingArt} aria-hidden="true">
        <Icon name="insert" size={28} />
      </span>
      <h1 className={styles.title}>{title}</h1>
      <p className={styles.lead}>{lead}</p>
      <div className={styles.missingActions}>{children}</div>
    </div>
  );
}

/** While the capture loads. */
function Loading() {
  return (
    <div className={styles.page} aria-busy="true">
      <Skeleton shape={SkeletonShape.Title} />
      <div className={styles.loading}>
        <Skeleton shape={SkeletonShape.Block} count={2} />
        <Skeleton shape={SkeletonShape.Block} />
      </div>
    </div>
  );
}

/** The choices for a capture's product (`?capture=`): its measured outline and height. */
function FromCapture({ captureId }: { readonly captureId: string }) {
  const query = useCapture(captureId);
  const capture = query.capture;
  const measured = capture.kind === "some" && capture.value.result.kind === "some" ? capture.value.result.value.products[0] : undefined;
  const product = useMemo(() => (measured === undefined ? none : some(measuredProduct(measured))), [measured]);
  const captureHref = `${AppRoute.Capture}/${encodeURIComponent(captureId)}`;

  if (capture.kind === "none") {
    if (query.error.kind === "some" && NOT_HERE.has(query.error.value.code)) {
      return (
        <NothingHere title={t("choices.missing.title")} lead={t("choices.missing.lead")}>
          <LinkButton href={AppRoute.Capture} size={ButtonSize.Large} withArrow>
            {t("capture.action.another")}
          </LinkButton>
        </NothingHere>
      );
    }
    if (query.error.kind === "some") {
      return (
        <div className={styles.missing}>
          <Notice
            tone={NoticeTone.Error}
            action={
              <Button variant={ButtonVariant.Secondary} onClick={query.reload}>
                {t("session.retry")}
              </Button>
            }
          >
            {describeError(query.error.value)}
          </Notice>
        </div>
      );
    }
    return <Loading />;
  }
  if (product.kind === "none") {
    const pending = capture.value.status === CaptureStatus.Pending;
    return (
      <NothingHere
        title={t(pending ? "choices.pending.title" : "choices.unmeasured.title")}
        lead={t(pending ? "choices.pending.lead" : "choices.unmeasured.lead")}
      >
        <LinkButton href={captureHref} size={ButtonSize.Large} withArrow>
          {t("choices.toCapture")}
        </LinkButton>
      </NothingHere>
    );
  }
  return (
    <ChoicesForm
      product={product.value}
      link={some({ capture: capture.value, apply: query.apply })}
      changeHref={captureHref}
    />
  );
}

/** The choices for the sizes typed in this tab (`features/capture/typed-product.ts`). */
function FromTyped() {
  const kept = useKeptTypedProduct();
  const product = useMemo(
    () => (kept.kind === "read" && kept.product.kind === "some" ? some(typedProduct(kept.product.value)) : none),
    [kept],
  );
  if (kept.kind === "reading") {
    return <Loading />;
  }
  if (product.kind === "none") {
    return (
      <NothingHere title={t("choices.nothing.title")} lead={t("choices.nothing.lead")}>
        <LinkButton href={AppRoute.Capture} size={ButtonSize.Large} withArrow>
          {t("workspace.start.action")}
        </LinkButton>
        <LinkButton href={`${AppRoute.Capture}?mode=typed`} variant={ButtonVariant.Secondary}>
          {t("capture.action.typed")}
        </LinkButton>
      </NothingHere>
    );
  }
  return <ChoicesForm product={product.value} link={none} changeHref={`${AppRoute.Capture}?mode=typed`} />;
}

/**
 * A new design (`/design/new`, MILESTONES M1-E4): from a capture (`?capture={id}`) or from the
 * sizes typed in this tab.
 */
export function DesignChoices() {
  const captureId = useSearchParams().get(CAPTURE_PARAM);
  return captureId === null || captureId === "" ? <FromTyped /> : <FromCapture key={captureId} captureId={captureId} />;
}
