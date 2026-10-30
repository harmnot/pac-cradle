import { describe, expect, it } from "vitest";
import { routeAfterSignIn, safeNextPath, withNext } from "./routes";
import { AccountRole, NextStep, Paper, type Session } from "./types";

function session(nextSteps: Session["nextSteps"]): Session {
  return {
    account: {
      id: "019a0000-5eed-7000-8000-0000000000b1",
      email: { kind: "some", value: "buyer@paccradle.test" },
      role: AccountRole.User,
      hasPassword: false,
      passkeyCount: 0,
      paperDefault: Paper.A4,
      createdAt: "2026-10-30T05:00:00Z",
    },
    nextSteps,
  };
}

describe("safeNextPath", () => {
  it("keeps paths on this site", () => {
    expect(safeNextPath("/account")).toBe("/account");
    expect(safeNextPath("/designs?page=2")).toBe("/designs?page=2");
  });

  it("sends anything else to the workspace", () => {
    for (const candidate of [null, "", "account", "https://evil.example", "//evil.example", "/\\evil.example"]) {
      expect(safeNextPath(candidate), String(candidate)).toBe("/app");
    }
  });
});

describe("withNext", () => {
  it("adds next, except for the workspace", () => {
    expect(withNext({ route: "/sign-in", next: "/account" })).toBe("/sign-in?next=%2Faccount");
    expect(withNext({ route: "/sign-in", next: "/app" })).toBe("/sign-in");
  });
});

describe("routeAfterSignIn", () => {
  it("asks for a password first, then a passkey, then goes back", () => {
    const both = session([NextStep.SetPassword, NextStep.AddPasskey]);
    expect(routeAfterSignIn({ session: both, next: "/account", canMakePasskey: true })).toBe(
      "/security/password?next=%2Faccount",
    );
    const passkeyOnly = session([NextStep.AddPasskey]);
    expect(routeAfterSignIn({ session: passkeyOnly, next: "/account", canMakePasskey: true })).toBe(
      "/security/passkey?next=%2Faccount",
    );
    expect(routeAfterSignIn({ session: session([]), next: "/account", canMakePasskey: true })).toBe(
      "/account",
    );
  });

  it("offers no passkey on a device that can't make one", () => {
    const passkeyOnly = session([NextStep.AddPasskey]);
    expect(routeAfterSignIn({ session: passkeyOnly, next: "/app", canMakePasskey: false })).toBe("/app");
  });
});
