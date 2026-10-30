import type { ApiError } from "./api-client";
import { hasMessage, t, type MessageVars } from "./i18n";

/** The message key of an API or client error code, e.g. `error.AUTH_LINK_INVALID`. */
function keyOf(code: string): string {
  return `error.${code}`;
}

/** The error's `meta` values a message can show (`{retry_after_seconds}`, `{min_length}`). */
function varsOf(error: ApiError): MessageVars {
  return Object.fromEntries(
    Object.entries(error.meta).filter(
      (entry): entry is [string, string | number] =>
        typeof entry[1] === "string" || typeof entry[1] === "number",
    ),
  );
}

/**
 * What to tell the person about a failed call: the app's own words for codes it knows
 * (`locales/en.json`, `error.<CODE>`), else the API's message (already in their language),
 * else a general apology.
 */
export function describeError(error: ApiError): string {
  const key = keyOf(error.code);
  if (hasMessage(key)) {
    return t(key, varsOf(error));
  }
  return error.message === "" ? t("error.generic") : error.message;
}
