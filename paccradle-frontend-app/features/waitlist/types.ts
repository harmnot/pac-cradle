/** Where on the site a waitlist sign-up came from (sent to the list tool and to analytics). */
export const WaitlistSource = {
  FakeDoor: "fake_door",
  Join: "join_section",
  Pricing: "pricing_page",
} as const;

export type WaitlistSource = (typeof WaitlistSource)[keyof typeof WaitlistSource];

/**
 * Background the waitlist form sits on. Lives here, not in the "use client" form module:
 * a server component importing a constant from a client module only gets a reference.
 */
export const WaitlistTone = {
  /** On paper (light surfaces). */
  Light: "light",
  /** On the forest-green join panel. */
  Dark: "dark",
} as const;

export type WaitlistTone = (typeof WaitlistTone)[keyof typeof WaitlistTone];

/** Why a sign-up failed. Each reason has a `waitlist.error.<reason>` message. */
export type WaitlistErrorReason = "not_configured" | "network" | "rejected";

/** Result of a sign-up attempt. */
export type WaitlistResult =
  | { readonly kind: "ok" }
  | { readonly kind: "error"; readonly reason: WaitlistErrorReason };

/** A sign-up request. */
export interface WaitlistRequest {
  readonly email: string;
  readonly source: WaitlistSource;
}
