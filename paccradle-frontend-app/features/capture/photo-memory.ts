import { none, some, type Maybe } from "@/lib/maybe";

/**
 * The photo this tab just uploaded, by capture id, so the capture page shows it at once
 * (and in development, where photos are kept in memory with no link a browser can open).
 * Only this tab's memory: never stored. After a reload the page uses the API's signed link.
 */

/** A photo held as an object URL, with its pixel size (the frame of `contour_px`). */
export interface HeldPhoto {
  readonly url: string;
  readonly width: number;
  readonly height: number;
}

const held = new Map<string, HeldPhoto>();
/** Photos kept at most; older ones are released. */
const KEEP = 4;

/** Keeps `blob` for `captureId`. */
export function holdPhoto({ captureId, blob, width, height }: { captureId: string; blob: Blob; width: number; height: number }): void {
  held.set(captureId, { url: URL.createObjectURL(blob), width, height });
  const extra = Array.from(held.keys()).slice(0, Math.max(held.size - KEEP, 0));
  extra.forEach((id) => {
    const photo = held.get(id);
    if (photo !== undefined) {
      URL.revokeObjectURL(photo.url);
    }
    held.delete(id);
  });
}

/** The photo held for `captureId`, if this tab uploaded it. */
export function heldPhoto(captureId: string): Maybe<HeldPhoto> {
  const photo = held.get(captureId);
  return photo === undefined ? none : some(photo);
}

/** A web link, as opposed to development's `memory://` photo links a browser can't open. */
const WEB_LINK = /^https?:\/\//;

/** A photo the page can show, with its pixel size when known (else read when it loads). */
export interface ShownPhoto {
  readonly url: string;
  readonly size: Maybe<{ readonly width: number; readonly height: number }>;
}

/** The photo to show for a capture: this tab's own upload, else the API's signed link when it is a web link. */
export function shownPhoto({ captureId, link }: { captureId: string; link: Maybe<string> }): Maybe<ShownPhoto> {
  const photo = heldPhoto(captureId);
  if (photo.kind === "some") {
    return some({ url: photo.value.url, size: some({ width: photo.value.width, height: photo.value.height }) });
  }
  return link.kind === "some" && WEB_LINK.test(link.value) ? some({ url: link.value, size: none }) : none;
}
