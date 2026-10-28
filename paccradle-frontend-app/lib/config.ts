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
}

const DEFAULT_PLAUSIBLE_SCRIPT_SRC = "https://plausible.io/js/script.js";

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
};
