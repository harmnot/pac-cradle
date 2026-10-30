import { none, some, type Maybe } from "@/lib/maybe";
import { PHOTO_MAX_LONG_SIDE, PHOTO_MIN_LONG_SIDE, type CameraExif } from "./types";

/**
 * Gets a photo ready to upload (CLAUDE.md §7.1): read the camera fields the measurement uses
 * from its EXIF (make, model, 35 mm focal length; never a location), draw it upright on a
 * canvas no larger than 4000 px, and re-encode it as a JPEG, which drops all its metadata.
 * Only Safari and WebKit can draw HEIC; elsewhere a HEIC file asks for a JPEG instead.
 */

/** A photo ready to upload. */
export interface PreparedPhoto {
  readonly blob: Blob;
  readonly width: number;
  readonly height: number;
  readonly exif: CameraExif;
}

/** Why a photo can't be used. */
export const PrepareFailure = {
  /** A HEIC file in a browser that can't draw one. */
  HeicUnsupported: "heic_unsupported",
  /** Not an image the browser can read. */
  Unreadable: "unreadable",
  /** Under 1000 px on its long side. */
  TooSmall: "too_small",
} as const;

export type PrepareFailure = (typeof PrepareFailure)[keyof typeof PrepareFailure];

export type PrepareResult =
  | { readonly kind: "ok"; readonly photo: PreparedPhoto }
  | { readonly kind: "failed"; readonly failure: PrepareFailure };

const HEIC_TYPES: ReadonlySet<string> = new Set(["image/heic", "image/heif"]);
const HEIC_NAME = /\.(heic|heif)$/i;
const JPEG = "image/jpeg";
const JPEG_QUALITY = 0.92;
const MAX_TEXT = 100;
/**
 * The EXIF tags the measurement uses, named per block: exifr's `lite` bundle can't `pick`
 * across blocks, and the GPS block is never even read.
 */
const EXIF_OPTIONS = {
  ifd0: { pick: ["Make", "Model"] },
  exif: { pick: ["FocalLengthIn35mmFormat"] },
  gps: false,
};

/** Whether a file is HEIC/HEIF (by type, or by name when the browser gives no type). */
export function isHeic(file: { readonly type: string; readonly name: string }): boolean {
  return HEIC_TYPES.has(file.type.toLowerCase()) || HEIC_NAME.test(file.name);
}

/** The size to draw at: scaled down so the long side is at most `max`, never up. */
export function targetSize({
  width,
  height,
  max = PHOTO_MAX_LONG_SIDE,
}: {
  width: number;
  height: number;
  max?: number;
}): { readonly width: number; readonly height: number } {
  const scale = Math.min(1, max / Math.max(width, height));
  return { width: Math.round(width * scale), height: Math.round(height * scale) };
}

function text(value: unknown): Maybe<string> {
  if (typeof value !== "string") {
    return none;
  }
  const trimmed = value.trim().slice(0, MAX_TEXT);
  return trimmed === "" ? none : some(trimmed);
}

/** The camera fields from what exifr read (anything else is ignored). */
export function cameraExif(tags: unknown): CameraExif {
  const record = typeof tags === "object" && tags !== null ? (tags as Record<string, unknown>) : {};
  const focal = record.FocalLengthIn35mmFormat;
  return {
    make: text(record.Make),
    model: text(record.Model),
    focalLength35mm: typeof focal === "number" && focal > 0 ? some(focal) : none,
  };
}

/**
 * The camera fields of a photo (a `File` in the browser, bytes in tests). A photo without
 * EXIF, or one exifr can't read, has none: the measurement then assumes a phone lens.
 */
export async function readCameraExif(input: Blob | Uint8Array): Promise<CameraExif> {
  try {
    const exifr = (await import("exifr/dist/lite.esm.mjs")).default;
    return cameraExif(await exifr.parse(input, EXIF_OPTIONS));
  } catch {
    return cameraExif({});
  }
}

async function decode(file: File): Promise<Maybe<ImageBitmap>> {
  try {
    return some(await createImageBitmap(file, { imageOrientation: "from-image" }));
  } catch {
    return none;
  }
}

async function encode({
  bitmap,
  width,
  height,
}: {
  bitmap: ImageBitmap;
  width: number;
  height: number;
}): Promise<Maybe<Blob>> {
  const canvas = document.createElement("canvas");
  canvas.width = width;
  canvas.height = height;
  const context = canvas.getContext("2d");
  if (context === null) {
    return none;
  }
  context.imageSmoothingQuality = "high";
  context.drawImage(bitmap, 0, 0, width, height);
  const blob = await new Promise<Blob | null>((resolve) => canvas.toBlob(resolve, JPEG, JPEG_QUALITY));
  return blob === null ? none : some(blob);
}

/** Reads, checks and re-encodes `file` for upload. */
export async function preparePhoto(file: File): Promise<PrepareResult> {
  const [exif, decoded] = await Promise.all([readCameraExif(file), decode(file)]);
  if (decoded.kind === "none") {
    return {
      kind: "failed",
      failure: isHeic(file) ? PrepareFailure.HeicUnsupported : PrepareFailure.Unreadable,
    };
  }
  const bitmap = decoded.value;
  try {
    if (Math.max(bitmap.width, bitmap.height) < PHOTO_MIN_LONG_SIDE) {
      return { kind: "failed", failure: PrepareFailure.TooSmall };
    }
    const size = targetSize({ width: bitmap.width, height: bitmap.height });
    const blob = await encode({ bitmap, ...size });
    if (blob.kind === "none") {
      return { kind: "failed", failure: PrepareFailure.Unreadable };
    }
    return { kind: "ok", photo: { blob: blob.value, ...size, exif } };
  } finally {
    bitmap.close();
  }
}
