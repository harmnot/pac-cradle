import { NextStep, type Account, type Session } from "./types";

/**
 * Where the app's sign-in screens live, and where a sign-in goes next: the steps the API
 * asked for (`next_steps`), then back to the page the person came from.
 */

/** The app's routes for sign-in and the account. */
export const AppRoute = {
  Workspace: "/app",
  Capture: "/capture",
  /** The design choices for a measured or typed product (M1-E4). */
  DesignNew: "/design/new",
  SignIn: "/sign-in",
  SignUp: "/sign-up",
  EmailLink: "/auth/link",
  Account: "/account",
  SetPassword: "/security/password",
  AddPasskey: "/security/passkey",
} as const;

export type AppRoute = (typeof AppRoute)[keyof typeof AppRoute];

/** The query parameter holding where to go after a sign-in. */
export const NEXT_PARAM = "next";

const SAME_ORIGIN_PATH = /^\/(?![/\\])/;

/**
 * `candidate` when it is a path on this site (`/…`, never `//host` or `/\host`, which
 * browsers read as another site), else the workspace.
 */
export function safeNextPath(candidate: string | null): string {
  if (candidate === null || !SAME_ORIGIN_PATH.test(candidate)) {
    return AppRoute.Workspace;
  }
  return candidate;
}

/** `route` with `?next=` set to `next` (left off when it is the workspace). */
export function withNext({ route, next }: { route: string; next: string }): string {
  if (next === AppRoute.Workspace) {
    return route;
  }
  return `${route}?${new URLSearchParams({ [NEXT_PARAM]: next }).toString()}`;
}

const PASSKEY_SKIP_KEY = "paccradle.passkey.skip";

/** Whether the person said not to offer a passkey on this device again. */
export function passkeySkippedHere(account: Account): boolean {
  try {
    return window.localStorage.getItem(`${PASSKEY_SKIP_KEY}.${account.id}`) === "1";
  } catch {
    return false;
  }
}

/**
 * Remembers on this device only (not a token or a secret, so `localStorage` is fine) that the
 * person doesn't want a passkey here.
 */
export function skipPasskeyHere(account: Account): void {
  try {
    window.localStorage.setItem(`${PASSKEY_SKIP_KEY}.${account.id}`, "1");
  } catch {
    // Storage is off (private mode): the offer comes back next time, which is harmless.
  }
}

/**
 * The page after a sign-in, or after one of its steps: set a password first, then offer a
 * passkey where the device can make one and the person hasn't declined it here, then `next`.
 */
export function routeAfterSignIn({
  session,
  next,
  canMakePasskey,
}: {
  session: Session;
  next: string;
  canMakePasskey: boolean;
}): string {
  const steps = session.nextSteps;
  if (steps.includes(NextStep.SetPassword)) {
    return withNext({ route: AppRoute.SetPassword, next });
  }
  const offerPasskey =
    steps.includes(NextStep.AddPasskey) &&
    canMakePasskey &&
    !passkeySkippedHere(session.account);
  if (offerPasskey) {
    return withNext({ route: AppRoute.AddPasskey, next });
  }
  return next;
}
