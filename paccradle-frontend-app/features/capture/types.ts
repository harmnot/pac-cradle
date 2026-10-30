import type { Maybe } from "@/lib/maybe";
import type { Paper } from "@/features/identity/types";

/**
 * Captures (photo → measured outline), written from the dated contracts
 * `captures-upload-2026-10-30` and `captures-vision-2026-10-30` and the vision result JSON
 * (CLAUDE.md §5.5). `Wire…` types are the JSON as sent; the rest are what the app uses.
 */

/** How the product is measured from the photo. */
export const CaptureMode = {
  /** The product itself on the paper (fastest). */
  Photo: "photo",
  /** A pen outline drawn round the product, product removed (most exact, §5.2 fix A). */
  Trace: "trace",
} as const;

export type CaptureMode = (typeof CaptureMode)[keyof typeof CaptureMode];

/** A capture's state. */
export const CaptureStatus = {
  /** A vision run is queued or running. */
  Pending: "pending",
  Done: "done",
  Failed: "failed",
} as const;

export type CaptureStatus = (typeof CaptureStatus)[keyof typeof CaptureStatus];

/** A product's shape from above. */
export type Shape = "round" | "rectangular" | "freeform";

/** A point in photo pixels or millimetres, `[x, y]`. */
export type Point = readonly [number, number];

/** What the browser read from the photo before re-encoding it (never a location). */
export interface CameraExif {
  readonly make: Maybe<string>;
  readonly model: Maybe<string>;
  /** EXIF `FocalLengthIn35mmFormat`, only when the photo has one above 0. */
  readonly focalLength35mm: Maybe<number>;
}

/** The settings sent with an upload. */
export interface CaptureSettings {
  readonly paper: Paper;
  readonly mode: CaptureMode;
  /** Trace mode's pen width. */
  readonly penWidthMm: Maybe<number>;
  readonly exif: CameraExif;
}

/** `POST /captures`' 201. */
export interface WireCaptureCreated {
  readonly capture_id: string;
  readonly status: CaptureStatus;
  readonly mode: CaptureMode;
  readonly paper: Paper;
  readonly photo: { readonly format: string; readonly width: number; readonly height: number };
  readonly created_at: string;
  readonly photo_kept_until: string;
}

/** Something vision tells about the photo, by `kind` (translated as `warning.<kind>`). */
export type PhotoWarning =
  | { readonly kind: "tall_product_looks_bigger" }
  | { readonly kind: "outline_may_be_bigger"; readonly height_mm: number; readonly up_to_mm: number }
  | { readonly kind: "touches_paper_edge" }
  | { readonly kind: "other_objects"; readonly count: number }
  | { readonly kind: "low_confidence"; readonly confidence: number };

/** One measured product (§5.5). */
export interface WireMeasuredProduct {
  readonly id: string;
  readonly shape: Shape;
  readonly confidence: number;
  readonly bbox_mm: { readonly l: number; readonly w: number };
  readonly diameter_mm: number | null;
  readonly angle_deg: number;
  readonly center_mm: Point;
  readonly area_mm2: number;
  readonly contour_mm: readonly Point[];
  readonly contour_px: readonly Point[];
  /** `user` once the outline was edited by hand (E3). */
  readonly contour_source: "detected" | "user";
  readonly height_mm: number | null;
  readonly height_source: string;
}

/** A vision run's result (§5.5). */
export interface WireVisionResult {
  readonly mode: CaptureMode;
  readonly paper: {
    readonly size: string;
    /** Top-left, top-right, bottom-right, bottom-left, in photo pixels. */
    readonly corners_px: readonly [Point, Point, Point, Point];
    readonly px_per_mm: number;
  };
  readonly products: readonly WireMeasuredProduct[];
  readonly warnings: readonly PhotoWarning[];
  readonly level_used: string;
}

/** The settings every run of a capture uses (`GET /captures/{capture_id}`'s `settings`). */
export interface WireCaptureSettings {
  readonly pen_width_mm: number | null;
  readonly height_mm: number | null;
  readonly focal_length_35mm: number | null;
  /** The user's last tap on the product, in photo pixels. */
  readonly tap_point_px: Point | null;
}

/** `GET /captures/{capture_id}`, and `POST /captures/{capture_id}/correct`'s 200 and 202. */
export interface WireCapture {
  readonly capture_id: string;
  readonly status: CaptureStatus;
  readonly mode: CaptureMode;
  readonly paper: Paper;
  readonly settings: WireCaptureSettings;
  readonly result: WireVisionResult | null;
  readonly error: {
    readonly code: string;
    readonly message: string;
    readonly meta: Readonly<Record<string, unknown>>;
  } | null;
  readonly photo: { readonly url: string; readonly url_expires_at: string } | null;
  readonly created_at: string;
  readonly photo_kept_until: string;
}

/** A capture, for the app. */
export interface Capture {
  readonly id: string;
  readonly status: CaptureStatus;
  readonly mode: CaptureMode;
  readonly paper: Paper;
  /** The latest outline: kept while a newer run is pending and after one fails. */
  readonly result: Maybe<WireVisionResult>;
  readonly error: Maybe<{
    readonly code: string;
    readonly message: string;
    readonly meta: Readonly<Record<string, unknown>>;
  }>;
  /**
   * A signed link to the photo and when it stops working (ms since the epoch): 10 minutes;
   * fetch the capture again for a new one.
   */
  readonly photo: Maybe<{ readonly url: string; readonly expiresAt: number }>;
  /** Where the user last tapped the product, in photo pixels. */
  readonly tapPoint: Maybe<Point>;
}

/** A correction to a capture's outline (`POST /captures/{capture_id}/correct`, M1-E3). */
export type Correction =
  /** A tap on the product: vision runs again and keeps the outline under it (202). */
  | { readonly kind: "tap"; readonly point: Point }
  /** The outline as the user drew it, measured as drawn at once (200). */
  | { readonly kind: "edit"; readonly points: readonly Point[] }
  /**
   * The product's height (M1-E4): kept in the capture's settings and applied to its result at
   * once (200; 202 while a run is pending), rechecking the tall-product warning.
   */
  | { readonly kind: "height"; readonly mm: number };

/** Upload limits the browser checks before sending (the API checks them again). */
export const PHOTO_MAX_LONG_SIDE = 4000;
export const PHOTO_MIN_LONG_SIDE = 1000;
/** Trace mode's pen widths to choose from, and the default (§5.2 fix A). */
export const PEN_WIDTHS_MM = [0.3, 0.5, 0.7, 1.0] as const;
export const DEFAULT_PEN_WIDTH_MM = 0.5;
