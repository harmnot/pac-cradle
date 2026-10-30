import { describe, expect, it } from "vitest";
import { none, some } from "@/lib/maybe";
import {
  clampToPaper,
  crossingEdges,
  enclosingCircle,
  hitOutline,
  liveSize,
  onPaper,
  paperMapOf,
  polygonArea,
  project,
  toPaperMm,
  type PaperMap,
} from "./geometry";
import {
  editOutline,
  isChanged,
  MIN_OUTLINE_POINTS,
  outlineToSend,
  startEditing,
  type EditorAction,
  type EditorState,
} from "./outline-editor";
import { CaptureMode, CaptureStatus, type Capture, type Point, type WireVisionResult } from "./types";
import { keepPhotoLink } from "./use-capture";

/** The paper of the contract's example result: an A4 sheet seen a little skewed. */
const PAPER: WireVisionResult["paper"] = {
  size: "A4",
  corners_px: [
    [1180.2, 200.4],
    [2859.8, 240.1],
    [2979.6, 2799.7],
    [1050.3, 2760.2],
  ],
  px_per_mm: 10,
};

const SQUARE: readonly Point[] = [
  [0, 0],
  [10, 0],
  [10, 10],
  [0, 10],
];

function mapOf(paper: WireVisionResult["paper"]): PaperMap {
  const map = paperMapOf(paper);
  if (map.kind === "none") {
    throw new Error("no paper map");
  }
  return map.value;
}

function near({ point, expected }: { point: Point; expected: Point }): void {
  expect(point[0]).toBeCloseTo(expected[0], 6);
  expect(point[1]).toBeCloseTo(expected[1], 6);
}

describe("the photo ↔ paper mapping", () => {
  const map = mapOf(PAPER);

  it("takes each corner of the sheet in the photo to the sheet's corner in mm", () => {
    const sheet: readonly Point[] = [
      [0, 0],
      [210, 0],
      [210, 297],
      [0, 297],
    ];
    PAPER.corners_px.forEach((corner, index) => {
      const mm = toPaperMm({ map, point: corner });
      expect(mm.kind).toBe("some");
      if (mm.kind === "some") {
        near({ point: mm.value, expected: sheet[index] });
      }
    });
  });

  it("goes back to the same photo point", () => {
    const photo: Point = [2015.3, 1268.8];
    const mm = toPaperMm({ map, point: photo });
    const back = mm.kind === "some" ? project({ transform: map.toPhoto, point: mm.value }) : mm;
    expect(back.kind).toBe("some");
    if (back.kind === "some") {
      near({ point: back.value, expected: photo });
    }
  });

  it("knows Letter, and refuses a sheet it doesn't know", () => {
    expect(mapOf({ ...PAPER, size: "Letter" }).size).toEqual({ width: 215.9, height: 279.4 });
    expect(paperMapOf({ ...PAPER, size: "A3" }).kind).toBe("none");
  });

  it("says whether a photo point is on the sheet, as vision does", () => {
    expect(onPaper({ map, point: [2000, 1500] })).toBe(true);
    expect(onPaper({ map, point: [900, 1500] })).toBe(false);
    expect(onPaper({ map, point: [2000, 150] })).toBe(false);
  });

  it("moves a point off the sheet onto its edge, and leaves one on it alone", () => {
    const inside: Point = [2000, 1500];
    expect(clampToPaper({ map, point: inside })).toBe(inside);
    const moved = clampToPaper({ map, point: [700, 1500] });
    expect(onPaper({ map, point: moved })).toBe(true);
    const mm = toPaperMm({ map, point: moved });
    expect(mm.kind === "some" && mm.value[0]).toBeCloseTo(0.05, 6);
  });
});

describe("checking an edited outline", () => {
  it("measures a polygon's area whichever way round it runs", () => {
    expect(polygonArea(SQUARE)).toBe(100);
    expect(polygonArea([...SQUARE].reverse())).toBe(100);
  });

  it("finds no crossing in a simple or dented outline, nor in a triangle", () => {
    expect(crossingEdges(SQUARE)).toEqual([]);
    expect(
      crossingEdges([
        [0, 0],
        [10, 0],
        [5, 4],
        [10, 10],
        [0, 10],
      ]),
    ).toEqual([]);
    expect(
      crossingEdges([
        [0, 0],
        [10, 0],
        [0, 10],
      ]),
    ).toEqual([]);
  });

  it("names the edges of a bow tie and of a point dragged onto another edge", () => {
    expect(
      crossingEdges([
        [0, 0],
        [10, 10],
        [10, 0],
        [0, 10],
      ]),
    ).toEqual([0, 2]);
    expect(
      crossingEdges([
        [0, 0],
        [10, 0],
        [5, 10],
        [10, 10],
        [0, 10],
      ]).length,
    ).toBeGreaterThan(0);
  });

  it("sizes a rotated rectangle by its own sides, and calls a circle round", () => {
    const angle = Math.PI / 6;
    const rotate = ([x, y]: Point): Point => [
      x * Math.cos(angle) - y * Math.sin(angle),
      x * Math.sin(angle) + y * Math.cos(angle),
    ];
    const box = liveSize(
      (
        [
          [0, 0],
          [80, 0],
          [80, 40],
          [0, 40],
        ] as const
      ).map(rotate),
    );
    expect(box.shape).toBe("box");
    if (box.shape === "box") {
      expect(box.lengthMm).toBeCloseTo(80, 6);
      expect(box.widthMm).toBeCloseTo(40, 6);
      expect(box.areaMm2).toBeCloseTo(3200, 6);
    }
    const circle = liveSize(
      Array.from({ length: 48 }, (_, index): Point => {
        const turn = (index / 48) * 2 * Math.PI;
        return [26 * Math.cos(turn), 26 * Math.sin(turn)];
      }),
    );
    expect(circle.shape).toBe("round");
    if (circle.shape === "round") {
      expect(circle.diameterMm).toBeCloseTo(52, 6);
    }
  });

  it("finds the smallest circle round the points, as OpenCV does", () => {
    expect(enclosingCircle(SQUARE).radius).toBeCloseTo(Math.SQRT2 * 5, 9);
    // An obtuse triangle: the circle on its longest side, not through all three.
    const obtuse = enclosingCircle([
      [0, 0],
      [10, 0],
      [5, 1],
    ]);
    expect(obtuse.radius).toBeCloseTo(5, 9);
    expect(obtuse.center[0]).toBeCloseTo(5, 9);
    expect(enclosingCircle([[3, 4]]).radius).toBe(0);
  });

  it("picks the nearest point, else a spot on an edge, else nothing", () => {
    expect(hitOutline({ points: SQUARE, pointer: [9.5, 0.4], pointReach: 1, edgeReach: 1 })).toEqual({
      kind: "point",
      index: 1,
    });
    expect(hitOutline({ points: SQUARE, pointer: [5, 0.5], pointReach: 1, edgeReach: 1 })).toEqual({
      kind: "edge",
      after: 0,
      at: [5, 0],
    });
    expect(hitOutline({ points: SQUARE, pointer: [5, 5], pointReach: 1, edgeReach: 1 })).toEqual({
      kind: "nothing",
    });
  });
});

/** Runs actions from a fresh editor of the square. */
function run(actions: readonly EditorAction[]): EditorState {
  return actions.reduce(editOutline, startEditing(SQUARE));
}

describe("the outline editor", () => {
  it("drags a point in one undo step, and undoes and redoes it", () => {
    const dragged = run([
      { type: "grab", index: 2 },
      { type: "drag", to: [11, 11] },
      { type: "drag", to: [12, 12] },
      { type: "release" },
    ]);
    expect(dragged.points[2]).toEqual([12, 12]);
    expect(dragged.past).toHaveLength(1);
    expect(isChanged(dragged)).toBe(true);
    const undone = editOutline(dragged, { type: "undo" });
    expect(undone.points).toEqual(SQUARE);
    expect(isChanged(undone)).toBe(false);
    expect(editOutline(undone, { type: "redo" }).points[2]).toEqual([12, 12]);
  });

  it("leaves no undo step for a point picked up and put down where it was", () => {
    const clicked = run([{ type: "grab", index: 1 }, { type: "release" }]);
    expect(clicked.past).toHaveLength(0);
    expect(clicked.selected).toEqual(some(1));
  });

  it("adds a point after an edge's first point and drags it", () => {
    const added = run([
      { type: "insert", after: 0, at: [5, 0] },
      { type: "drag", to: [5, -2] },
      { type: "release" },
    ]);
    expect(added.points).toEqual([[0, 0], [5, -2], [10, 0], [10, 10], [0, 10]]);
    expect(added.selected).toEqual(some(1));
    expect(added.past).toHaveLength(1);
  });

  it("removes the selected point, but never below 3 points", () => {
    const fewer = run([{ type: "select", index: some(0) }, { type: "remove" }]);
    expect(fewer.points).toHaveLength(3);
    const still = editOutline({ ...fewer, selected: some(0) }, { type: "remove" });
    expect(still.points).toHaveLength(MIN_OUTLINE_POINTS);
  });

  it("joins keyboard nudges of one point into one undo step", () => {
    const nudged = run([
      { type: "select", index: some(3) },
      { type: "nudge", to: [0, 11] },
      { type: "nudge", to: [0, 12] },
      { type: "nudge", to: [0, 13] },
    ]);
    expect(nudged.points[3]).toEqual([0, 13]);
    expect(nudged.past).toHaveLength(1);
    const other = editOutline(editOutline(nudged, { type: "step", by: 1 }), { type: "nudge", to: [1, 1] });
    expect(other.selected).toEqual(some(0));
    expect(other.past).toHaveLength(2);
  });

  it("steps round the points both ways", () => {
    expect(run([{ type: "step", by: 1 }]).selected).toEqual(some(0));
    expect(run([{ type: "step", by: -1 }]).selected).toEqual(some(3));
    expect(run([{ type: "select", index: some(3) }, { type: "step", by: 1 }]).selected).toEqual(some(0));
  });

  it("resets to the outline it began with, as one undo step", () => {
    const reset = run([
      { type: "grab", index: 0 },
      { type: "drag", to: [-1, -1] },
      { type: "release" },
      { type: "reset" },
    ]);
    expect(reset.points).toEqual(SQUARE);
    expect(editOutline(reset, { type: "undo" }).points[0]).toEqual([-1, -1]);
  });

  it("begins afresh with new points and no history", () => {
    const begun = editOutline(run([{ type: "select", index: some(0) }, { type: "remove" }]), {
      type: "begin",
      points: SQUARE,
    });
    expect(begun.points).toEqual(SQUARE);
    expect(begun.past).toEqual([]);
    expect(begun.selected.kind).toBe("none");
  });

  it("sends points in photo pixels to a hundredth", () => {
    expect(outlineToSend([[1.23456, 9.87654]])).toEqual([[1.23, 9.88]]);
  });
});

describe("polling a capture", () => {
  const NOW = Date.parse("2026-10-30T10:00:00Z");
  const capture = (url: string, expiresInMs: number): Capture => ({
    id: "c1",
    status: CaptureStatus.Pending,
    mode: CaptureMode.Photo,
    paper: "a4",
    result: none,
    error: none,
    photo: some({ url, expiresAt: NOW + expiresInMs }),
    tapPoint: none,
  });

  it("keeps the photo link on screen while it works, so the photo isn't fetched again", () => {
    const kept = keepPhotoLink({ shown: some(capture("first", 9 * 60_000)), next: capture("second", 10 * 60_000), now: NOW });
    expect(kept.photo).toEqual(some({ url: "first", expiresAt: NOW + 9 * 60_000 }));
  });

  it("takes the new link when the old one is about to stop working, or there was none", () => {
    const fresh = capture("second", 10 * 60_000);
    expect(keepPhotoLink({ shown: some(capture("first", 30_000)), next: fresh, now: NOW })).toBe(fresh);
    expect(keepPhotoLink({ shown: none, next: fresh, now: NOW })).toBe(fresh);
  });
});
