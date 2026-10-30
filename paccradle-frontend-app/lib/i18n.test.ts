import { describe, expect, it } from "vitest";
import { describeError } from "./errors";
import { formatRelative, tCount } from "./i18n";

describe("tCount", () => {
  it("picks zero, one or other", () => {
    expect(tCount({ base: "account.security.passkeyCount", count: 0 })).toBe("None yet");
    expect(tCount({ base: "account.security.passkeyCount", count: 1 })).toBe("1 passkey");
    expect(tCount({ base: "account.security.passkeyCount", count: 3 })).toBe("3 passkeys");
  });

  it("falls back to other when there is no zero", () => {
    expect(tCount({ base: "design.products", count: 0 })).toBe("0 products");
  });
});

describe("describeError", () => {
  it("uses the app's words, filled from the error's meta", () => {
    const error = {
      status: 429,
      code: "RATE_LIMITED",
      message: "Too many requests.",
      meta: { retry_after_seconds: 12 },
    };
    expect(describeError(error)).toBe("Too many tries. Please wait 12 s and try again.");
  });

  it("falls back to the API's message, then to an apology", () => {
    const unknown = { status: 400, code: "SOMETHING_NEW", message: "From the API.", meta: {} };
    expect(describeError(unknown)).toBe("From the API.");
    expect(describeError({ ...unknown, message: "" })).toBe(
      "Something went wrong on our side. Please try again.",
    );
  });
});

describe("formatRelative", () => {
  const now = Date.parse("2026-10-30T12:00:00Z");

  it("says how long ago", () => {
    expect(formatRelative({ timestamp: "2026-10-27T12:00:00Z", now })).toBe("3 days ago");
    expect(formatRelative({ timestamp: "2026-10-30T11:00:00Z", now })).toBe("1 hour ago");
    expect(formatRelative({ timestamp: "2026-10-30T11:59:40Z", now })).toBe("now");
  });
});
