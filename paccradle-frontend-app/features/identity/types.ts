import type { Maybe } from "@/lib/maybe";

/**
 * Sign-in and account types, written from the dated contracts in `paccradle-backend/postman/`
 * (`auth-2026-10-29`, `billing-me-2026-10-30`). `Wire…` types are the JSON as sent; the rest
 * are what the app works with (`Maybe` instead of `null`).
 */

/** What an account acts as. A guest has no confirmed email yet. */
export const AccountRole = {
  Guest: "guest",
  User: "user",
  Admin: "admin",
} as const;

export type AccountRole = (typeof AccountRole)[keyof typeof AccountRole];

/** What the web app should offer right after a sign-in. */
export const NextStep = {
  /** No password yet: ask for one (`PUT /me/password`). */
  SetPassword: "set_password",
  /** The sign-in didn't use a passkey: offer to make one on this device, if it can. */
  AddPasskey: "add_passkey",
} as const;

export type NextStep = (typeof NextStep)[keyof typeof NextStep];

/** Paper sizes the capture flow measures on. */
export const Paper = {
  A4: "a4",
  Letter: "letter",
} as const;

export type Paper = (typeof Paper)[keyof typeof Paper];

/** Plans (`GET /me`, `billing.plan`). */
export const Plan = {
  Free: "free",
  Pro: "pro",
  Studio: "studio",
} as const;

export type Plan = (typeof Plan)[keyof typeof Plan];

/** An account as the API sends it. */
export interface WireAccount {
  readonly id: string;
  readonly email: string | null;
  readonly role: AccountRole;
  readonly has_password: boolean;
  readonly passkey_count: number;
  readonly locale: string;
  readonly paper_default: Paper;
  readonly created_at: string;
}

/** A session body: every sign-in and refresh answers with one. */
export interface WireSession {
  readonly access_token: string;
  readonly token_type: "Bearer";
  /** Seconds the access token lasts (900 on the web app). */
  readonly expires_in: number;
  readonly account: WireAccount;
  readonly next_steps: readonly NextStep[];
}

/** A subscription as `GET /me` sends it. */
export interface WireSubscription {
  readonly plan: string;
  readonly status: string;
  readonly current_period_end: string | null;
}

/** `GET /me`: the account, plus what it has paid for (M1-D8). */
export interface WireMe extends WireAccount {
  readonly billing: {
    readonly plan: Plan;
    readonly credits: number;
    readonly subscription: WireSubscription | null;
    readonly unlocked_designs: number;
  };
}

/** The signed-in person (or guest). */
export interface Account {
  readonly id: string;
  readonly email: Maybe<string>;
  readonly role: AccountRole;
  readonly hasPassword: boolean;
  readonly passkeyCount: number;
  readonly paperDefault: Paper;
  readonly createdAt: string;
}

/** A session the app holds: the token stays in `lib/api-client`, never in React state. */
export interface Session {
  readonly account: Account;
  readonly nextSteps: readonly NextStep[];
}

/** A subscription, as the account page shows it. */
export interface Subscription {
  readonly plan: string;
  readonly status: string;
  readonly currentPeriodEnd: Maybe<string>;
}

/** What an account has paid for. */
export interface Billing {
  readonly plan: Plan;
  readonly credits: number;
  readonly subscription: Maybe<Subscription>;
  readonly unlockedDesigns: number;
}

/** `GET /me`, for the app. */
export interface Me {
  readonly account: Account;
  readonly billing: Billing;
}

/** `POST /auth/passkey/sign-in/options`: W3C `PublicKeyCredentialRequestOptionsJSON`. */
export interface WirePasskeySignInOptions {
  readonly ceremony_id: string;
  readonly public_key: PublicKeyCredentialRequestOptionsJSON;
}

/** `POST /me/passkeys/options`: W3C `PublicKeyCredentialCreationOptionsJSON`. */
export interface WirePasskeyRegistrationOptions {
  readonly ceremony_id: string;
  readonly public_key: PublicKeyCredentialCreationOptionsJSON;
}

/** Password limits (`AUTH_PASSWORD_REJECTED` names the same). */
export const PASSWORD_MIN_LENGTH = 12;
export const PASSWORD_MAX_LENGTH = 128;
