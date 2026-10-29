"use client";

import { useEffect, useState } from "react";
import { t, type MessageKey } from "@/lib/i18n";
import { none, some, type Maybe } from "@/lib/maybe";
import { loadInsertCore, NO_SHEET, type InsertCore } from "../insert-core";
import { InsertStyle, sampleDesign } from "../samples";
import { pathData } from "../svg";
import type { FitColor, Packaging, Piece, Point, ProductFit } from "../types";
import styles from "./EngineCheck.module.css";

/**
 * Development check for the browser engine (M1-A10): generates the sample design on every
 * slider move, draws the insert's pieces, and times each frame (generate + fits). The
 * benchmark sweeps the slider through many frames for steady numbers.
 */

const DIAMETER = { min: 40, max: 100, step: 0.5, start: 72 } as const;
/** Frames the running median covers, and frames one benchmark runs. */
const RECENT_FRAMES = 60;
const BENCH_FRAMES = 200;
const BENCH_WARNING_RUNS = 20;
const PERCENTILE_95 = 0.95;
/** Space round and between the drawn pieces, in mm. */
const DRAWING_GAP_MM = 8;
const STYLES: readonly InsertStyle[] = [InsertStyle.Grid, InsertStyle.Card, InsertStyle.Cradle];
const FIT_COLORS: readonly FitColor[] = ["green", "yellow", "red"];

type EngineState =
  | { readonly kind: "loading" }
  | { readonly kind: "failed"; readonly message: string }
  | { readonly kind: "ready"; readonly core: InsertCore; readonly loadMs: number };

/** One frame's outcome. */
type Frame =
  | { readonly kind: "ok"; readonly packaging: Packaging; readonly fits: readonly ProductFit[] }
  | { readonly kind: "error"; readonly message: string };

/** A frame and how long it took. */
interface TimedFrame {
  readonly frame: Frame;
  readonly ms: number;
}

interface BenchResult {
  readonly medianMs: number;
  readonly p95Ms: number;
  readonly warningsMs: number;
}

/** The slider and style the frames are drawn for. */
interface Inputs {
  readonly style: InsertStyle;
  readonly diameterMm: number;
}

const STYLE_LABELS: Readonly<Record<InsertStyle, MessageKey>> = {
  [InsertStyle.Grid]: "dev.engine.style.partition_grid",
  [InsertStyle.Card]: "dev.engine.style.cutout_card",
  [InsertStyle.Cradle]: "dev.engine.style.cradle_platform",
};

/** Live engine check: style, jar size, timing, and the flat pieces. */
export function EngineCheck() {
  const [engine, setEngine] = useState<EngineState>({ kind: "loading" });
  const [inputs, setInputs] = useState<Inputs>({
    style: InsertStyle.Grid,
    diameterMm: DIAMETER.start,
  });
  const [latest, setLatest] = useState<Maybe<TimedFrame>>(none);
  const [recentMs, setRecentMs] = useState<readonly number[]>([]);
  const [bench, setBench] = useState<Maybe<BenchResult>>(none);

  /** Runs a frame for `next` and keeps its timing. */
  const show = ({ core, next }: { readonly core: InsertCore; readonly next: Inputs }) => {
    const timed = runFrame({ core, ...next });
    setInputs(next);
    setLatest(some(timed));
    setRecentMs((previous) => [...previous, timed.ms].slice(-RECENT_FRAMES));
  };

  useEffect(() => {
    const start = async () => {
      const startedAt = performance.now();
      const result = await loadInsertCore();
      if (result.kind === "error") {
        setEngine({ kind: "failed", message: result.message });
        return;
      }
      setEngine({ kind: "ready", core: result.core, loadMs: performance.now() - startedAt });
      const timed = runFrame({
        core: result.core,
        style: InsertStyle.Grid,
        diameterMm: DIAMETER.start,
      });
      setLatest(some(timed));
      setRecentMs([timed.ms]);
    };
    void start();
  }, []);

  if (engine.kind === "loading") {
    return <p className={styles.status}>{t("dev.engine.loading")}</p>;
  }
  if (engine.kind === "failed") {
    return <p className={styles.error}>{t("dev.engine.loadFailed", { message: engine.message })}</p>;
  }

  const { core } = engine;
  return (
    <section className={styles.check}>
      <p className={styles.status}>
        {t("dev.engine.loaded", { ms: engine.loadMs.toFixed(0), size: wasmSize() })}
      </p>
      <fieldset className={styles.controls}>
        <legend>{t("dev.engine.style")}</legend>
        {STYLES.map((option) => (
          <label key={option} className={styles.choice}>
            <input
              type="radio"
              name="style"
              value={option}
              checked={inputs.style === option}
              onChange={() => show({ core, next: { ...inputs, style: option } })}
            />
            {t(STYLE_LABELS[option])}
          </label>
        ))}
      </fieldset>
      <label className={styles.slider}>
        {t("dev.engine.diameter", { mm: inputs.diameterMm.toFixed(1) })}
        <input
          type="range"
          min={DIAMETER.min}
          max={DIAMETER.max}
          step={DIAMETER.step}
          value={inputs.diameterMm}
          onChange={(event) =>
            show({ core, next: { ...inputs, diameterMm: Number(event.target.value) } })
          }
        />
      </label>
      {latest.kind === "some" && (
        <p className={styles.timing}>
          {t("dev.engine.frame", {
            ms: latest.value.ms.toFixed(2),
            count: recentMs.length,
            median: percentile({ values: recentMs, share: 0.5 }).toFixed(2),
          })}
        </p>
      )}
      <p className={styles.timing}>
        <button
          type="button"
          className={styles.button}
          onClick={() => setBench(some(benchmark({ core, style: inputs.style })))}
        >
          {t("dev.engine.bench", { frames: BENCH_FRAMES })}
        </button>{" "}
        {bench.kind === "some" &&
          t("dev.engine.benchResult", {
            frames: BENCH_FRAMES,
            median: bench.value.medianMs.toFixed(2),
            p95: bench.value.p95Ms.toFixed(2),
            warnings: bench.value.warningsMs.toFixed(2),
          })}
      </p>
      {latest.kind === "some" && <FrameView frame={latest.value.frame} />}
    </section>
  );
}

/** The frame's fit counts and drawing, or the engine's message. */
function FrameView({ frame }: { readonly frame: Frame }) {
  if (frame.kind === "error") {
    return <p className={styles.error}>{t("dev.engine.error", { message: frame.message })}</p>;
  }
  const counts = Object.fromEntries(
    FIT_COLORS.map((color) => [color, frame.fits.filter((fit) => fit.color === color).length]),
  );
  return (
    <>
      <p className={styles.timing}>{t("dev.engine.fits", counts)}</p>
      <PiecesDrawing pieces={frame.packaging.insert.pieces} />
    </>
  );
}

/** Each insert piece flat, side by side: cut lines solid, creases dashed. */
function PiecesDrawing({ pieces }: { readonly pieces: readonly Piece[] }) {
  const placed = layOut(pieces);
  const width = placed.reduce((end, item) => Math.max(end, item.x + item.width), 0);
  const height = placed.reduce((tallest, item) => Math.max(tallest, item.height), 0);
  return (
    <figure className={styles.figure}>
      <svg
        className={styles.drawing}
        viewBox={`0 0 ${width + DRAWING_GAP_MM} ${height + 2 * DRAWING_GAP_MM}`}
        role="img"
        aria-label={t("dev.engine.drawing")}
      >
        {placed.map(({ piece, x, minX, minY }) => (
          <g key={piece.name} transform={`translate(${x - minX} ${DRAWING_GAP_MM - minY})`}>
            {piece.creases.map((crease, index) => (
              <path key={index} className={styles.crease} d={pathData(crease)} />
            ))}
            {[piece.outline, ...piece.holes].map((cut, index) => (
              <path key={index} className={styles.cut} d={pathData(cut)} />
            ))}
          </g>
        ))}
      </svg>
      <figcaption>{t("dev.engine.drawing")}</figcaption>
    </figure>
  );
}

/** Where each piece goes in the row, from its outline's bounds. */
function layOut(pieces: readonly Piece[]) {
  let x = DRAWING_GAP_MM;
  return pieces.map((piece) => {
    const points = outlinePoints(piece);
    const xs = points.map((point) => point[0]);
    const ys = points.map((point) => point[1]);
    const [minX, minY] = [Math.min(...xs), Math.min(...ys)];
    const width = Math.max(...xs) - minX;
    const item = { piece, x, minX, minY, width, height: Math.max(...ys) - minY };
    x += width + DRAWING_GAP_MM;
    return item;
  });
}

/** The outline's corner points; the preview's paths are straight lines only. */
function outlinePoints(piece: Piece): readonly Point[] {
  const ends = piece.outline.segments.map((segment) =>
    segment.kind === "line" ? segment.to : segment.center,
  );
  return [piece.outline.start, ...ends];
}

/** Generates and fit-checks the sample once, timed. */
function runFrame({
  core,
  style,
  diameterMm,
}: {
  readonly core: InsertCore;
  readonly style: InsertStyle;
  readonly diameterMm: number;
}): TimedFrame {
  const design = sampleDesign({ style, diameterMm });
  const startedAt = performance.now();
  const packaging = core.generate(design);
  const fits = core.fits(design);
  const ms = performance.now() - startedAt;
  if (packaging.kind === "error") {
    return { frame: packaging, ms };
  }
  if (fits.kind === "error") {
    return { frame: fits, ms };
  }
  return { frame: { kind: "ok", packaging: packaging.value, fits: fits.value }, ms };
}

/** Sweeps the jar's size through the slider's range, a frame per step, and times it. */
function benchmark({
  core,
  style,
}: {
  readonly core: InsertCore;
  readonly style: InsertStyle;
}): BenchResult {
  const span = DIAMETER.max - DIAMETER.min;
  const sizes = Array.from(
    { length: BENCH_FRAMES },
    (_, frame) => DIAMETER.min + (span * frame) / BENCH_FRAMES,
  );
  const frameMs = sizes.map((diameterMm) => runFrame({ core, style, diameterMm }).ms);
  const warningMs = sizes.slice(0, BENCH_WARNING_RUNS).map((diameterMm) => {
    const design = sampleDesign({ style, diameterMm });
    const startedAt = performance.now();
    core.warnings({ design, options: NO_SHEET });
    return performance.now() - startedAt;
  });
  return {
    medianMs: percentile({ values: frameMs, share: 0.5 }),
    p95Ms: percentile({ values: frameMs, share: PERCENTILE_95 }),
    warningsMs: percentile({ values: warningMs, share: 0.5 }),
  };
}

/** The value `share` of the way through the sorted values (0.5: the median). */
function percentile({
  values,
  share,
}: {
  readonly values: readonly number[];
  readonly share: number;
}): number {
  const sorted = [...values].sort((a, b) => a - b);
  return sorted[Math.min(sorted.length - 1, Math.floor(sorted.length * share))] ?? 0;
}

/** The downloaded WebAssembly file's size, as the browser recorded it. */
function wasmSize(): string {
  const entry = performance
    .getEntriesByType("resource")
    .find((resource) => resource.name.includes("insert_core_bg"));
  if (!(entry instanceof PerformanceResourceTiming)) {
    return t("dev.engine.sizeUnknown");
  }
  return t("dev.engine.size", {
    transfer: kilobytes(entry.transferSize),
    decoded: kilobytes(entry.decodedBodySize),
  });
}

function kilobytes(bytes: number): string {
  return (bytes / 1024).toFixed(0);
}
