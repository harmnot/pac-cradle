import { fromEnv, valueOr, type Maybe } from "./maybe";

/**
 * Public configuration, read from NEXT_PUBLIC_* environment variables at build time.
 * Every value here ships to the browser, so never put secrets in it. See `.env.example`.
 */
export interface PublicConfig {
  /** Operator's legal name for the Terms (Paddle domain approval, CLAUDE.md §9.1). */
  readonly legalName: Maybe<string>;
  /** Support and legal contact address. */
  readonly contactEmail: Maybe<string>;
  /** Form endpoint of the email-list tool that stores waitlist sign-ups. */
  readonly waitlistEndpoint: Maybe<string>;
  /** Site domain as configured in Plausible; analytics are off when missing. */
  readonly plausibleDomain: Maybe<string>;
  /** Plausible script URL (change it when self-hosting Plausible). */
  readonly plausibleScriptSrc: string;
  /**
   * The API's origin, without a trailing slash. It must be on the web app's own site
   * (`app.paccradle.com` next to `api.paccradle.com`; `localhost` next to `localhost` in
   * development), or browsers drop the refresh cookie (MILESTONES M1-D3).
   */
  readonly apiUrl: string;
}

const DEFAULT_PLAUSIBLE_SCRIPT_SRC = "https://plausible.io/js/script.js";

/** `scripts/dev-api.sh`, on the same site as `next dev` (`localhost`, not `127.0.0.1`). */
const DEFAULT_API_URL = "http://localhost:7773";
const TRAILING_SLASHES = /\/+$/;

/** First year shown in the copyright line. */
export const COPYRIGHT_YEAR = 2026;

export const publicConfig: PublicConfig = {
  legalName: fromEnv(process.env.NEXT_PUBLIC_LEGAL_NAME),
  contactEmail: fromEnv(process.env.NEXT_PUBLIC_CONTACT_EMAIL),
  waitlistEndpoint: fromEnv(process.env.NEXT_PUBLIC_WAITLIST_ENDPOINT),
  plausibleDomain: fromEnv(process.env.NEXT_PUBLIC_PLAUSIBLE_DOMAIN),
  plausibleScriptSrc: valueOr(
    fromEnv(process.env.NEXT_PUBLIC_PLAUSIBLE_SCRIPT_SRC),
    DEFAULT_PLAUSIBLE_SCRIPT_SRC,
  ),
  apiUrl: valueOr(fromEnv(process.env.NEXT_PUBLIC_API_URL), DEFAULT_API_URL).replace(
    TRAILING_SLASHES,
    "",
  ),
};
