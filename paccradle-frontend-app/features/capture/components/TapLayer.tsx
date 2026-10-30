"use client";

import { useState, type CSSProperties, type MouseEvent, type PointerEvent } from "react";
import { none, type Maybe } from "@/lib/maybe";
import type { Point } from "../types";
import type { FigureView } from "./OutlineFigure";
import styles from "./TapLayer.module.css";

/** The pin's sizes on screen, in CSS pixels. */
const PIN_PX = 7;
const RETICLE_PX = 20;
const TICK_PX = 7;

/** What a pin on the photo is showing. */
export const PinState = {
  /** Sent: vision is measuring again where it points. */
  Sending: "sending",
  /** Not sent: the tap was off the paper. */
  Refused: "refused",
} as const;

export type PinState = (typeof PinState)[keyof typeof PinState];

/** A pin where the person tapped, rippling while vision measures there. */
export function TapPin({
  view,
  point,
  state,
}: {
  readonly view: FigureView;
  readonly point: Point;
  readonly state: PinState;
}) {
  const radius = PIN_PX * view.unitsPerPx;
  const [x, y] = point;
  return (
    // `--px`: one screen pixel in photo pixels, for the shake (CSS lengths in SVG are user units).
    <g
      className={styles.pin}
      data-state={state}
      aria-hidden="true"
      style={{ "--px": `${view.unitsPerPx}px` } as CSSProperties}
    >
      <circle className={styles.ripple} cx={x} cy={y} r={radius} />
      <circle className={`${styles.ripple} ${styles.rippleLate}`} cx={x} cy={y} r={radius} />
      <circle className={styles.dot} cx={x} cy={y} r={radius} vectorEffect="non-scaling-stroke" />
    </g>
  );
}

/** Props for the tap layer. */
export interface TapLayerProps {
  readonly view: FigureView;
  /** The pin of the last tap, if any. */
  readonly pin: Maybe<{ readonly point: Point; readonly state: PinState }>;
  /** Taps on the photo, in photo pixels. Not called while a tap is being sent. */
  readonly onTap: (point: Point) => void;
}

/** Whether a point lies in the frame (taps in the letterbox round the photo don't count). */
function inFrame({ view, point: [x, y] }: { view: FigureView; point: Point }): boolean {
  const { frame } = view;
  return x >= frame.x && x <= frame.x + frame.width && y >= frame.y && y <= frame.y + frame.height;
}

/**
 * Tap the product (MILESTONES M1-E3): a reticle follows the mouse over the photo, a tap
 * drops a pin and hands the spot (photo pixels) to `onTap`. Taps are clicks, so a finger
 * scrolling the page past the photo doesn't tap.
 */
export function TapLayer({ view, pin, onTap }: TapLayerProps) {
  const [reticle, setReticle] = useState<Maybe<Point>>(none);
  const sending = pin.kind === "some" && pin.value.state === PinState.Sending;
  const { frame } = view;

  const onPointerMove = (event: PointerEvent<SVGRectElement>) => {
    if (event.pointerType !== "mouse") {
      return;
    }
    const point = view.toPhoto(event);
    setReticle(point.kind === "some" && inFrame({ view, point: point.value }) ? point : none);
  };

  const onClick = (event: MouseEvent<SVGRectElement>) => {
    const point = view.toPhoto(event);
    if (sending || point.kind === "none" || !inFrame({ view, point: point.value })) {
      return;
    }
    onTap(point.value);
  };

  const ring = RETICLE_PX * view.unitsPerPx;
  const tick = TICK_PX * view.unitsPerPx;
  return (
    <g className={styles.layer} data-sending={sending}>
      {reticle.kind === "some" && !sending && (
        <g className={styles.reticle} aria-hidden="true">
          <circle cx={reticle.value[0]} cy={reticle.value[1]} r={ring} vectorEffect="non-scaling-stroke" />
          <path
            d={[
              `M${reticle.value[0]} ${reticle.value[1] - ring - tick}v${tick * 1.6}`,
              `M${reticle.value[0]} ${reticle.value[1] + ring + tick}v${-tick * 1.6}`,
              `M${reticle.value[0] - ring - tick} ${reticle.value[1]}h${tick * 1.6}`,
              `M${reticle.value[0] + ring + tick} ${reticle.value[1]}h${-tick * 1.6}`,
            ].join("")}
            vectorEffect="non-scaling-stroke"
          />
        </g>
      )}
      {pin.kind === "some" && (
        // Keyed by the spot, so each new tap drops a new pin.
        <TapPin key={pin.value.point.join(",")} view={view} point={pin.value.point} state={pin.value.state} />
      )}
      <rect
        className={styles.catcher}
        x={frame.x}
        y={frame.y}
        width={frame.width}
        height={frame.height}
        onPointerMove={onPointerMove}
        onPointerLeave={() => setReticle(none)}
        onClick={onClick}
      />
    </g>
  );
}
