import en from "@/locales/en.json";

/** Every UI message key. English is the only language at launch (DEC-11). */
export type MessageKey = keyof typeof en;

/** Values for `{placeholders}` in a message. */
export type MessageVars = Readonly<Record<string, string | number>>;

const PLACEHOLDER = /\{(\w+)\}/g;
const LOCALE = "en-US";
const CURRENCY = "USD";
const CENTS_PER_DOLLAR = 100;

/**
 * Returns the UI text for a key, with `{placeholders}` filled from `vars`.
 * Unknown placeholders are left as they are so they show up in review.
 */
export function t(key: MessageKey, vars: MessageVars = {}): string {
  return en[key].replace(PLACEHOLDER, (match, name: string) =>
    name in vars ? String(vars[name]) : match,
  );
}

/** Formats a USD amount given in cents, without decimals for whole dollars ($10, $139). */
export function formatUsd(cents: number): string {
  const whole = cents % CENTS_PER_DOLLAR === 0;
  return new Intl.NumberFormat(LOCALE, {
    style: "currency",
    currency: CURRENCY,
    minimumFractionDigits: whole ? 0 : 2,
    maximumFractionDigits: whole ? 0 : 2,
  }).format(cents / CENTS_PER_DOLLAR);
}

/** Formats an ISO date (YYYY-MM-DD) as a long date, e.g. "October 27, 2026". */
export function formatDate(isoDate: string): string {
  return new Intl.DateTimeFormat(LOCALE, { dateStyle: "long", timeZone: "UTC" }).format(
    new Date(`${isoDate}T00:00:00Z`),
  );
}
