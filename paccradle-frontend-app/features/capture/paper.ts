import { useSyncExternalStore } from "react";
import { Paper } from "@/features/identity/types";
import { none, some, type Maybe } from "@/lib/maybe";

/**
 * The paper the product stands on (CLAUDE.md §3 step 2): US Letter in the United States,
 * Canada, Mexico and the Philippines, A4 everywhere else. The region comes from the browser's
 * languages (`en-US`, `id-ID`); a choice the person made is remembered on this device.
 */

/** Regions that use US Letter. */
const LETTER_REGIONS: ReadonlySet<string> = new Set(["US", "CA", "MX", "PH"]);
const STORAGE_KEY = "paccradle.paper";
const TWO_LETTER_REGION = /^[A-Z]{2}$/;

/** A language tag's region, when it names a country (`en-US` → `US`; `en`, `es-419` → none). */
export function regionOf(tag: string): Maybe<string> {
  try {
    const region = new Intl.Locale(tag).region;
    return region !== undefined && TWO_LETTER_REGION.test(region) ? some(region) : none;
  } catch {
    return none;
  }
}

/** The paper for the first language that names a region; A4 when none does. */
export function paperForLanguages(languages: readonly string[]): Paper {
  const region = languages.map(regionOf).find((candidate) => candidate.kind === "some");
  return region?.kind === "some" && LETTER_REGIONS.has(region.value) ? Paper.Letter : Paper.A4;
}

function isPaper(value: string | null): value is Paper {
  return value === Paper.A4 || value === Paper.Letter;
}

/** The paper to offer first: the one chosen here before, else the region's. Browser only. */
export function defaultPaper(): Paper {
  try {
    const stored = window.localStorage.getItem(STORAGE_KEY);
    if (isPaper(stored)) {
      return stored;
    }
  } catch {
    // Storage is off: fall back to the region.
  }
  return paperForLanguages(navigator.languages.length > 0 ? navigator.languages : [navigator.language]);
}

/** Remembers the person's paper on this device (a preference, not personal data). */
export function rememberPaper(paper: Paper): void {
  try {
    window.localStorage.setItem(STORAGE_KEY, paper);
  } catch {
    // Not remembered; the region's default comes back next time.
  }
}

/** Nothing to listen for: the default changes only through the page's own choice. */
function subscribeNever(): () => void {
  return () => undefined;
}

function serverPaper(): Paper {
  return Paper.A4;
}

/**
 * The default paper in a component: A4 while the server renders and React hydrates the page,
 * then this browser's (`defaultPaper`), so the server's HTML and the first client render
 * agree and the radio buttons show the paper the upload will use.
 */
export function useDefaultPaper(): Paper {
  return useSyncExternalStore(subscribeNever, defaultPaper, serverPaper);
}
