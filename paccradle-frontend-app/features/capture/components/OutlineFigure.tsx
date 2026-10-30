"use client";

import { useCallback, useEffect, useState, type CSSProperties, type ReactNode } from "react";
import { t } from "@/lib/i18n";
import { none, some, type Maybe } from "@/lib/maybe";
import { closedPath } from "../geometry";
import type { Point, WireVisionResult } from "../types";
import styles from "./OutlineFigure.module.css";

/** A pixel size. */
interface Size {
  readonly width: number;
  readonly height: number;
}

/** A frame in photo pixels: where the drawing starts and how big it is. */
export interface Frame extends Size {
  readonly x: number;
  readonly y: number;
}

/** How far round the paper and outline the drawing reaches when there is no photo. */
const PAD = 0.06;
/** The frame before anything is known. */
const UNKNOWN_FRAME: Frame = { x: 0, y: 0, width: 4, height: 3 };
/** Line widths are counted in thousandths of the drawing's larger side. */
const STROKE_UNITS = 1000;

/** What a tool drawn over the figure knows about it (MILESTONES M1-E3). */
export interface FigureView {
  /** The drawing's frame, in photo pixels. */
  readonly frame: Frame;
  /** Photo pixels per screen pixel, for handles and reaches that stay the same size on screen. */
  readonly unitsPerPx: number;
  /** The photo shown, for the loupe (none while it loads or without one). */
  readonly photoUrl: Maybe<string>;
  /** Where a pointer at `clientX`/`clientY` is, in photo pixels. */
  readonly toPhoto: (client: { readonly clientX: number; readonly clientY: number }) => Maybe<Point>;
}

/** A tool on the figure: how the photo looks under it (`data-tool`). */
export const FigureTool = {
  None: "none",
  /** Tapping the product: a crosshair over the photo. */
  Tap: "tap",
  /** Moving the outline's points: the photo dims a little so they stand out. */
  Adjust: "adjust",
} as const;

export type FigureTool = (typeof FigureTool)[keyof typeof FigureTool];

/** How the products' outlines appear. */
export const OutlineEntrance = {
  /** Drawn round, then filled (a new measurement). */
  Draw: "draw",
  /** Shown at once with a short glow (an outline the user just drew). */
  Settle: "settle",
} as const;

export type OutlineEntrance = (typeof OutlineEntrance)[keyof typeof OutlineEntrance];

/** Props for the figure. */
export interface OutlineFigureProps {
  /** A photo the browser can show (this tab's upload, or an https link), with its size if known. */
  readonly image: Maybe<{ readonly url: string; readonly size: Maybe<Size> }>;
  readonly result: Maybe<WireVisionResult>;
  /** Vision is measuring: a scanning beam and corner brackets. */
  readonly measuring: boolean;
  /** The run failed: the photo dims. */
  readonly dimmed?: boolean;
  readonly tool?: FigureTool;
  /** Leave the products' outlines out (the editor draws its own). */
  readonly hideProducts?: boolean;
  readonly entrance?: OutlineEntrance;
  /** A tool drawn over the photo, in photo pixels, once the frame is known. */
  readonly overlay?: (view: FigureView) => ReactNode;
}

/** The box round the paper and every outline, padded: the drawing's frame without a photo. */
function boundsOf(result: WireVisionResult): Frame {
  const points = [...result.paper.corners_px, ...result.products.flatMap((product) => product.contour_px)];
  const xs = points.map(([x]) => x);
  const ys = points.map(([, y]) => y);
  const [minX, maxX, minY, maxY] = [Math.min(...xs), Math.max(...xs), Math.min(...ys), Math.max(...ys)];
  const pad = Math.max(maxX - minX, maxY - minY) * PAD;
  return { x: minX - pad, y: minY - pad, width: maxX - minX + 2 * pad, height: maxY - minY + 2 * pad };
}

/** A key that changes when an outline does, so a new one gets its entrance. */
function outlineKey({ id, points }: { id: string; points: readonly Point[] }): string {
  const middle = points[Math.floor(points.length / 2)] ?? [0, 0];
  return `${id}:${points.length}:${points[0]?.join(",") ?? ""}:${middle.join(",")}`;
}

/**
 * The capture as the person sees it: their photo with the found paper (dashed sage) and the
 * product's outline (terracotta, drawn in) on top, both in the photo's own pixels, the frame
 * `contour_px` uses. Without a photo the browser can show (development keeps photos in
 * memory; after 30 days they are deleted), the paper and outline are drawn on their own.
 * A tool (tap, point editor) draws over it in the same frame through `overlay`.
 */
export function OutlineFigure({
  image,
  result,
  measuring,
  dimmed = false,
  tool = FigureTool.None,
  hideProducts = false,
  entrance = OutlineEntrance.Draw,
  overlay,
}: OutlineFigureProps) {
  const [loaded, setLoaded] = useState<Maybe<Size>>(none);
  const [broken, setBroken] = useState(false);
  const [scale, setScale] = useState<Maybe<number>>(none);
  // The drawing's element, held as state: tools read it in their event handlers.
  const [svg, setSvg] = useState<Maybe<SVGSVGElement>>(none);
  const svgRef = useCallback((element: SVGSVGElement | null) => setSvg(element === null ? none : some(element)), []);
  const showImage = image.kind === "some" && !broken;
  const imageSize = image.kind === "some" && image.value.size.kind === "some" ? image.value.size : loaded;
  const known = showImage ? imageSize.kind === "some" : result.kind === "some";

  const frame: Frame =
    showImage && imageSize.kind === "some"
      ? { x: 0, y: 0, ...imageSize.value }
      : result.kind === "some" && !showImage
        ? boundsOf(result.value)
        : UNKNOWN_FRAME;
  const style = { aspectRatio: `${frame.width} / ${frame.height}` } as CSSProperties;
  // Line widths in the drawing's own units: a thousandth of its larger side.
  const lines = { "--unit": Math.max(frame.width, frame.height) / STROKE_UNITS } as CSSProperties;

  // Screen pixels per photo pixel, as the drawing is fitted (`meet`) into the figure.
  useEffect(() => {
    if (svg.kind === "none") {
      return;
    }
    const element = svg.value;
    const measure = () => {
      const box = element.getBoundingClientRect();
      const fitted = Math.min(box.width / frame.width, box.height / frame.height);
      setScale(fitted > 0 ? some(fitted) : none);
    };
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(element);
    return () => observer.disconnect();
  }, [svg, frame.width, frame.height]);

  const toPhoto = useCallback(
    ({ clientX, clientY }: { readonly clientX: number; readonly clientY: number }): Maybe<Point> => {
      const matrix = svg.kind === "some" ? svg.value.getScreenCTM() : null;
      if (matrix === null) {
        return none;
      }
      const point = new DOMPoint(clientX, clientY).matrixTransform(matrix.inverse());
      return some([point.x, point.y]);
    },
    [svg],
  );

  const view: Maybe<FigureView> =
    known && scale.kind === "some"
      ? some({
          frame,
          unitsPerPx: 1 / scale.value,
          photoUrl: showImage && imageSize.kind === "some" ? some(image.value.url) : none,
          toPhoto,
        })
      : none;

  return (
    <figure
      className={styles.figure}
      style={style}
      data-measuring={measuring}
      data-dimmed={dimmed}
      data-tool={tool}
    >
      {showImage && (
        // The person's own photo (an object URL or a short-lived signed link): next/image
        // can't optimise either.
        // eslint-disable-next-line @next/next/no-img-element
        <img
          src={image.value.url}
          alt={t("capture.figure.photoAlt")}
          className={styles.photo}
          onLoad={(event) =>
            setLoaded(some({ width: event.currentTarget.naturalWidth, height: event.currentTarget.naturalHeight }))
          }
          onError={() => setBroken(true)}
        />
      )}
      {!showImage && result.kind === "none" && <span className={styles.placeholder} aria-hidden="true" />}
      <svg
        ref={svgRef}
        className={styles.overlay}
        style={lines}
        viewBox={`${frame.x} ${frame.y} ${frame.width} ${frame.height}`}
        preserveAspectRatio="xMidYMid meet"
        // A picture, until a tool makes parts of it controls.
        role={tool === FigureTool.None ? "img" : "group"}
        aria-label={result.kind === "some" ? t("capture.figure.outlineAlt") : t("capture.figure.measuringAlt")}
      >
        {result.kind === "some" && known && (
          <>
            <path
              className={showImage ? styles.paperLine : styles.paperSheet}
              d={closedPath(result.value.paper.corners_px)}
              pathLength={1}
            />
            {!hideProducts &&
              result.value.products.map((product) => (
                <path
                  key={outlineKey({ id: product.id, points: product.contour_px })}
                  className={entrance === OutlineEntrance.Settle ? styles.outlineSettled : styles.outline}
                  d={closedPath(product.contour_px)}
                  pathLength={1}
                />
              ))}
          </>
        )}
        {overlay !== undefined && view.kind === "some" && overlay(view.value)}
      </svg>
      {measuring && (
        <>
          <span className={styles.beam} aria-hidden="true" />
          <span className={styles.brackets} aria-hidden="true">
            <span />
            <span />
            <span />
            <span />
          </span>
        </>
      )}
    </figure>
  );
}
