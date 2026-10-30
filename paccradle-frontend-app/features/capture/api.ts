import { apiJson, HttpMethod, type ApiResult } from "@/lib/api-client";
import { none, some } from "@/lib/maybe";
import type { Capture, CaptureSettings, Correction, WireCapture, WireCaptureCreated } from "./types";

/**
 * Capture calls (contracts `captures-upload-2026-10-30`, `captures-vision-2026-10-30`).
 * Uploads need a session: a visitor's guest session starts first (`useSession().ensureSession`).
 */

/** The upload's `capture` part: snake_case, optional values left out. */
function settingsJson(settings: CaptureSettings): Record<string, unknown> {
  const exif: Record<string, unknown> = {};
  if (settings.exif.make.kind === "some") exif.make = settings.exif.make.value;
  if (settings.exif.model.kind === "some") exif.model = settings.exif.model.value;
  if (settings.exif.focalLength35mm.kind === "some") {
    exif.focal_length_35mm = settings.exif.focalLength35mm.value;
  }
  const json: Record<string, unknown> = { paper: settings.paper, mode: settings.mode, exif };
  if (settings.penWidthMm.kind === "some") {
    json.pen_width_mm = settings.penWidthMm.value;
  }
  return json;
}

/** Uploads the re-encoded photo with its settings; vision starts measuring at once. */
export async function uploadCapture({
  photo,
  settings,
}: {
  photo: Blob;
  settings: CaptureSettings;
}): Promise<ApiResult<WireCaptureCreated>> {
  const form = new FormData();
  form.append(
    "capture",
    new Blob([JSON.stringify(settingsJson(settings))], { type: "application/json" }),
  );
  form.append("photo", photo, "photo.jpg");
  return apiJson<WireCaptureCreated>({ method: HttpMethod.Post, path: "/captures", body: form });
}

function toCapture(wire: WireCapture): Capture {
  return {
    id: wire.capture_id,
    status: wire.status,
    mode: wire.mode,
    paper: wire.paper,
    result: wire.result === null ? none : some(wire.result),
    error: wire.error === null ? none : some(wire.error),
    photo:
      wire.photo === null
        ? none
        : some({ url: wire.photo.url, expiresAt: Date.parse(wire.photo.url_expires_at) }),
    tapPoint: wire.settings.tap_point_px === null ? none : some(wire.settings.tap_point_px),
  };
}

/** The caller's capture (`CAPTURE_NOT_FOUND` for anyone else's). */
export async function getCapture({
  id,
  signal,
}: {
  id: string;
  signal?: AbortSignal;
}): Promise<ApiResult<Capture>> {
  const result = await apiJson<WireCapture>({ path: `/captures/${encodeURIComponent(id)}`, signal });
  return result.kind === "ok" ? { ...result, value: toCapture(result.value) } : result;
}

/** The correction's JSON: exactly one of `tap_point_px`, `edited_contour_px` or `height_mm`. */
function correctionJson(correction: Correction): Record<string, unknown> {
  switch (correction.kind) {
    case "tap":
      return { tap_point_px: correction.point };
    case "edit":
      return { edited_contour_px: correction.points };
    case "height":
      return { height_mm: correction.mm };
  }
}

/**
 * Corrects the capture's outline or adds the product's height. A tap answers 202 with a new
 * run pending (poll again); an edit answers 200, measured as drawn; a height answers 200 with
 * the warning rechecked (202 while a run is pending). Each answer is the whole capture.
 */
export async function correctCapture({
  id,
  correction,
}: {
  id: string;
  correction: Correction;
}): Promise<ApiResult<Capture>> {
  const result = await apiJson<WireCapture>({
    method: HttpMethod.Post,
    path: `/captures/${encodeURIComponent(id)}/correct`,
    body: correctionJson(correction),
  });
  return result.kind === "ok" ? { ...result, value: toCapture(result.value) } : result;
}
