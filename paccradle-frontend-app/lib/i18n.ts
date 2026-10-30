import en from "@/locales/en.json";

/** Every UI message key. English is the only language at launch (DEC-11). */
export type MessageKey = keyof typeof en;

/** Values for `{placeholders}` in a message. */
export type MessageVars = Readonly<Record<string, string | number>>;

const PLACEHOLDER = /\{(\w+)\}/g;
const LOCALE = "en-US";
const CURRENCY = "USD";
const CENTS_PER_DOLLAR = 100;

/** Whether `key` names a message (for keys built at run time, e.g. from an error code). */
export function hasMessage(key: string): key is MessageKey {
  return Object.hasOwn(en, key);
}

/**
 * Returns the UI text for a key, with `{placeholders}` filled from `vars`.
 * Unknown placeholders are left as they are so they show up in review.
 */
export function t(key: MessageKey, vars: MessageVars = {}): string {
  return en[key].replace(PLACEHOLDER, (match, name: string) =>
    name in vars ? String(vars[name]) : match,
  );
}

/**
 * A message that depends on a count: `<base>.zero` (when there is one and `count` is 0), else
 * `<base>.<plural category>` (`one`, `other` in English, from `Intl.PluralRules`), else
 * `<base>.other`. `{count}` is filled in.
 */
export function tCount({
  base,
  count,
  vars = {},
}: {
  base: string;
  count: number;
  vars?: MessageVars;
}): string {
  const values = { count, ...vars };
  const zero = `${base}.zero`;
  if (count === 0 && hasMessage(zero)) {
    return t(zero, values);
  }
  const exact = `${base}.${new Intl.PluralRules(LOCALE).select(count)}`;
  const other = `${base}.other`;
  if (hasMessage(exact)) {
    return t(exact, values);
  }
  return hasMessage(other) ? t(other, values) : base;
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

/** Formats an RFC 3339 timestamp from the API as a date, e.g. "Oct 30, 2026". */
export function formatTimestamp(timestamp: string): string {
  return new Intl.DateTimeFormat(LOCALE, { dateStyle: "medium" }).format(new Date(timestamp));
}

/** Units for relative times, largest first, with their length in seconds. */
const RELATIVE_UNITS: ReadonlyArray<readonly [Intl.RelativeTimeFormatUnit, number]> = [
  ["year", 31_536_000],
  ["month", 2_592_000],
  ["week", 604_800],
  ["day", 86_400],
  ["hour", 3_600],
  ["minute", 60],
];
const MS_PER_SECOND = 1000;

/** How long ago (or until) an RFC 3339 timestamp is, e.g. "3 days ago", "just now". */
export function formatRelative({ timestamp, now }: { timestamp: string; now: number }): string {
  const seconds = (new Date(timestamp).getTime() - now) / MS_PER_SECOND;
  const format = new Intl.RelativeTimeFormat(LOCALE, { numeric: "auto" });
  const unit = RELATIVE_UNITS.find(([, size]) => Math.abs(seconds) >= size);
  if (unit === undefined) {
    return format.format(0, "second");
  }
  const [name, size] = unit;
  return format.format(Math.round(seconds / size), name);
}

/** Formats an ISO date (YYYY-MM-DD) as a long date, e.g. "October 27, 2026". */
export function formatDate(isoDate: string): string {
  return new Intl.DateTimeFormat(LOCALE, { dateStyle: "long", timeZone: "UTC" }).format(
    new Date(`${isoDate}T00:00:00Z`),
  );
}
