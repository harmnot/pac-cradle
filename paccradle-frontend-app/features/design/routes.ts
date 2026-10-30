import { AppRoute } from "@/features/identity/routes";

/**
 * Where design pages live (M1-E4): the choices for a new design, from a capture or from typed
 * sizes, and a saved design.
 */

/** The query parameter naming the capture a new design starts from. */
export const CAPTURE_PARAM = "capture";

/** The design choices for a capture's product. */
export function newDesignFromCapture(captureId: string): string {
  return `${AppRoute.DesignNew}?${new URLSearchParams({ [CAPTURE_PARAM]: captureId }).toString()}`;
}

/** The design choices for the sizes typed in this tab. */
export const NEW_DESIGN_FROM_TYPED: string = AppRoute.DesignNew;

/** A saved design. */
export function designPath(designId: string): string {
  return `/design/${encodeURIComponent(designId)}`;
}
