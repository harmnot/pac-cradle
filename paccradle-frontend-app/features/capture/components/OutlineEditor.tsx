"use client";

import { useEffect, useId, useRef, useState, type CSSProperties, type KeyboardEvent, type PointerEvent } from "react";
import { t } from "@/lib/i18n";
import { none, some, type Maybe } from "@/lib/maybe";
import { clampToPaper, closedPath, hitOutline, type OutlineHit, type PaperMap } from "../geometry";
import type { Point } from "../types";
import type { OutlineEditing } from "../use-outline-editing";
import type { FigureView } from "./OutlineFigure";
import styles from "./OutlineEditor.module.css";

/** Sizes on screen, in CSS pixels: they stay the same however large the photo is shown. */
const HANDLE_PX = 5;
/** Handles shrink to this, at the least, where points crowd together on screen. */
const HANDLE_MIN_PX = 2.5;
/** A handle's radius as a share of the usual gap between points on screen. */
const HANDLE_SHARE_OF_GAP = 0.32;
const SELECTED_PX = 7.5;
/** How near a point or the line a pointer must be to take it; a finger gets more room. */
const POINT_REACH_PX = { mouse: 12, touch: 22 } as const;
const EDGE_REACH_PX = { mouse: 9, touch: 16 } as const;
/** Arrow keys move a point this far; with Shift, ten times as far. */
const NUDGE_PX = 1;
const NUDGE_FAR = 10;
/** The loupe over a dragged point: its radius, how much nearer it shows, its gap above the finger. */
const LOUPE_RADIUS_PX = 58;
const LOUPE_ZOOM = 3;
const LOUPE_GAP_PX = 34;
/** The handles pop in one after another over about this long, then stay put. */
const ENTRANCE_MS = 450;
const ENTRANCE_DONE_MS = 900;

const NOTHING: OutlineHit = { kind: "nothing" };

const ARROWS: Readonly<Record<string, Point>> = {
  ArrowLeft: [-1, 0],
  ArrowRight: [1, 0],
  ArrowUp: [0, -1],
  ArrowDown: [0, 1],
};

/** Props for the editor. */
export interface OutlineEditorProps {
  readonly view: FigureView;
  readonly editing: OutlineEditing;
  /** The sheet, to keep points on it (vision refuses points off it). */
  readonly map: Maybe<PaperMap>;
}

/** The handles' radius on screen: smaller where the points crowd (a small photo, a round outline). */
function handleRadiusPx({ points, unitsPerPx }: { points: readonly Point[]; unitsPerPx: number }): number {
  const gaps = points
    .map(([x, y], index) => {
      const [nx, ny] = points[(index + 1) % points.length];
      return Math.hypot(nx - x, ny - y) / unitsPerPx;
    })
    .sort((a, b) => a - b);
  const median = gaps[Math.floor(gaps.length / 2)] ?? 0;
  return Math.min(Math.max(median * HANDLE_SHARE_OF_GAP, HANDLE_MIN_PX), HANDLE_PX);
}

/** A finger gets wider reaches than a mouse or a pen. */
function reachFor(pointerType: string): "mouse" | "touch" {
  return pointerType === "touch" ? "touch" : "mouse";
}

/** The loupe: the photo round the dragged point, three times nearer, just above the finger. */
function Loupe({ view, points, point }: { readonly view: FigureView; readonly points: readonly Point[]; readonly point: Point }) {
  const clipId = useId();
  if (view.photoUrl.kind === "none") {
    return null;
  }
  const units = view.unitsPerPx;
  const radius = LOUPE_RADIUS_PX * units;
  const gap = LOUPE_GAP_PX * units;
  const { frame } = view;
  const [x, y] = point;
  const above = y - radius - gap;
  const cy = above - radius < frame.y ? y + radius + gap : above;
  const cx = Math.min(Math.max(x, frame.x + radius), frame.x + frame.width - radius);
  const cross = radius * 0.22;
  return (
    <g className={styles.loupe} aria-hidden="true">
      <defs>
        <clipPath id={clipId}>
          <circle cx={cx} cy={cy} r={radius} />
        </clipPath>
      </defs>
      <circle className={styles.loupeBack} cx={cx} cy={cy} r={radius} />
      <g clipPath={`url(#${clipId})`}>
        <g transform={`translate(${cx} ${cy}) scale(${LOUPE_ZOOM}) translate(${-x} ${-y})`}>
          <image href={view.photoUrl.value} x={0} y={0} width={frame.width} height={frame.height} />
          <path className={styles.loupeLine} d={closedPath(points)} vectorEffect="non-scaling-stroke" />
        </g>
      </g>
      <path
        className={styles.loupeCross}
        d={`M${cx - cross} ${cy}H${cx + cross}M${cx} ${cy - cross}V${cy + cross}`}
        vectorEffect="non-scaling-stroke"
      />
      <circle className={styles.loupeRing} cx={cx} cy={cy} r={radius} vectorEffect="non-scaling-stroke" />
    </g>
  );
}

/**
 * The outline's points over the photo (MILESTONES M1-E3), in photo pixels: drag a point to
 * move it, press on the line to add one and drag it out, double-click a point to remove it.
 * Points stay on the paper. From the keyboard (the figure focused): `[` and `]` pick a point,
 * the arrows move it (Shift: faster), Delete removes it, Escape lets go. While a point moves,
 * a loupe above it shows the photo three times nearer.
 */
export function OutlineEditor({ view, editing, map }: OutlineEditorProps) {
  const { state, dispatch, crossing } = editing;
  const [hover, setHover] = useState<OutlineHit>(NOTHING);
  const [entering, setEntering] = useState(true);
  const groupRef = useRef<SVGGElement>(null);
  const units = view.unitsPerPx;
  const dragging = state.drag.kind === "some";

  useEffect(() => {
    const timer = setTimeout(() => setEntering(false), ENTRANCE_DONE_MS);
    groupRef.current?.focus({ preventScroll: true });
    return () => clearTimeout(timer);
  }, []);

  const place = (point: Point): Point => (map.kind === "some" ? clampToPaper({ map: map.value, point }) : point);

  const hitAt = ({ point, pointerType }: { point: Point; pointerType: string }) => {
    const reach = reachFor(pointerType);
    return hitOutline({
      points: state.points,
      pointer: point,
      pointReach: POINT_REACH_PX[reach] * units,
      edgeReach: EDGE_REACH_PX[reach] * units,
    });
  };

  const onPointerDown = (event: PointerEvent<SVGRectElement>) => {
    if (event.button !== 0) {
      return;
    }
    const point = view.toPhoto(event);
    if (point.kind === "none") {
      return;
    }
    const hit = hitAt({ point: point.value, pointerType: event.pointerType });
    groupRef.current?.focus({ preventScroll: true });
    switch (hit.kind) {
      case "point":
        dispatch({ type: "grab", index: hit.index });
        break;
      case "edge":
        dispatch({ type: "insert", after: hit.after, at: place(hit.at) });
        break;
      case "nothing":
        dispatch({ type: "select", index: none });
        return;
    }
    event.preventDefault();
    event.currentTarget.setPointerCapture(event.pointerId);
    setHover(NOTHING);
  };

  const onPointerMove = (event: PointerEvent<SVGRectElement>) => {
    const point = view.toPhoto(event);
    if (point.kind === "none") {
      return;
    }
    if (dragging) {
      dispatch({ type: "drag", to: place(point.value) });
      return;
    }
    if (event.pointerType === "mouse") {
      setHover(hitAt({ point: point.value, pointerType: event.pointerType }));
    }
  };

  const onPointerUp = () => {
    if (dragging) {
      dispatch({ type: "release" });
    }
  };

  const onDoubleClick = (event: { readonly clientX: number; readonly clientY: number }) => {
    const point = view.toPhoto(event);
    if (point.kind === "none") {
      return;
    }
    const hit = hitAt({ point: point.value, pointerType: "mouse" });
    if (hit.kind === "point") {
      dispatch({ type: "select", index: some(hit.index) });
      dispatch({ type: "remove" });
    }
  };

  const onKeyDown = (event: KeyboardEvent<SVGGElement>) => {
    const arrow = ARROWS[event.key];
    if (arrow !== undefined) {
      event.preventDefault();
      if (state.selected.kind === "none") {
        dispatch({ type: "step", by: 1 });
        return;
      }
      const [x, y] = state.points[state.selected.value];
      const step = NUDGE_PX * units * (event.shiftKey ? NUDGE_FAR : 1);
      dispatch({ type: "nudge", to: place([x + arrow[0] * step, y + arrow[1] * step]) });
      return;
    }
    switch (event.key) {
      case "[":
        event.preventDefault();
        dispatch({ type: "step", by: -1 });
        return;
      case "]":
        event.preventDefault();
        dispatch({ type: "step", by: 1 });
        return;
      case "Delete":
      case "Backspace":
        event.preventDefault();
        dispatch({ type: "remove" });
        return;
      case "Escape":
        if (state.selected.kind === "some") {
          event.preventDefault();
          dispatch({ type: "select", index: none });
        }
        return;
    }
  };

  const { frame } = view;
  const count = state.points.length;
  const selectedPoint = state.selected.kind === "some" ? state.points[state.selected.value] : undefined;
  const handleRadius = handleRadiusPx({ points: state.points, unitsPerPx: units }) * units;
  const hoverTarget = dragging ? "drag" : hover.kind;

  return (
    <g
      ref={groupRef}
      className={styles.editor}
      tabIndex={0}
      role="application"
      aria-roledescription={t("capture.adjust.roleDescription")}
      aria-label={t("capture.adjust.editorLabel")}
      data-hover={hoverTarget}
      data-entering={entering}
      onKeyDown={onKeyDown}
    >
      <rect className={styles.focusRing} x={frame.x} y={frame.y} width={frame.width} height={frame.height} />
      <path className={styles.shape} d={closedPath(state.points)} vectorEffect="non-scaling-stroke" />
      {crossing.map((index) => {
        const [x1, y1] = state.points[index];
        const [x2, y2] = state.points[(index + 1) % count];
        return (
          <line
            key={index}
            className={styles.crossing}
            x1={x1}
            y1={y1}
            x2={x2}
            y2={y2}
            vectorEffect="non-scaling-stroke"
          />
        );
      })}
      {hover.kind === "edge" && (
        <g className={styles.ghost} aria-hidden="true">
          <circle cx={hover.at[0]} cy={hover.at[1]} r={SELECTED_PX * units} vectorEffect="non-scaling-stroke" />
          <path
            d={`M${hover.at[0] - 3 * units} ${hover.at[1]}h${6 * units}M${hover.at[0]} ${hover.at[1] - 3 * units}v${6 * units}`}
            vectorEffect="non-scaling-stroke"
          />
        </g>
      )}
      <g className={styles.handles} aria-hidden="true">
        {state.points.map(([x, y], index) => (
          <circle
            key={index}
            className={hover.kind === "point" && hover.index === index ? styles.handleHover : styles.handle}
            cx={x}
            cy={y}
            r={handleRadius}
            vectorEffect="non-scaling-stroke"
            style={{ "--delay": `${Math.round((index / count) * ENTRANCE_MS)}ms` } as CSSProperties}
          />
        ))}
      </g>
      {selectedPoint !== undefined && (
        <g className={styles.selected} data-dragging={dragging} aria-hidden="true">
          <circle className={styles.halo} cx={selectedPoint[0]} cy={selectedPoint[1]} r={SELECTED_PX * 2 * units} />
          <circle
            className={styles.selectedHandle}
            cx={selectedPoint[0]}
            cy={selectedPoint[1]}
            r={SELECTED_PX * units}
            vectorEffect="non-scaling-stroke"
          />
        </g>
      )}
      {dragging && selectedPoint !== undefined && <Loupe view={view} points={state.points} point={selectedPoint} />}
      <rect
        className={styles.catcher}
        x={frame.x}
        y={frame.y}
        width={frame.width}
        height={frame.height}
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerUp}
        onPointerCancel={onPointerUp}
        onPointerLeave={() => setHover(NOTHING)}
        onDoubleClick={onDoubleClick}
      />
    </g>
  );
}
