import {
  apiJson,
  apiNoContent,
  dropSession,
  holdSession,
  HttpMethod,
  refreshSession,
  type ApiResult,
  type NoContent,
} from "@/lib/api-client";
import { none, some, type Maybe } from "@/lib/maybe";
import type { AuthenticationResponse, RegistrationResponse } from "./passkey";
import type {
  Account,
  Me,
  Session,
  WireAccount,
  WireMe,
  WirePasskeyRegistrationOptions,
  WirePasskeySignInOptions,
  WireSession,
} from "./types";

/**
 * Sign-in and account calls (contract `auth-2026-10-29`, `/me` billing from
 * `billing-me-2026-10-30`). Calls that answer with a session hold its token
 * (`lib/api-client`) and give back the account and next steps only.
 */

function maybe<T>(value: T | null): Maybe<T> {
  return value === null ? none : some(value);
}

/** The app's account from the API's. */
export function toAccount(wire: WireAccount): Account {
  return {
    id: wire.id,
    email: maybe(wire.email),
    role: wire.role,
    hasPassword: wire.has_password,
    passkeyCount: wire.passkey_count,
    paperDefault: wire.paper_default,
    createdAt: wire.created_at,
  };
}

/** The app's session from a session body (its token is held, not returned). */
export function toSession(wire: WireSession): Session {
  return { account: toAccount(wire.account), nextSteps: wire.next_steps };
}

function mapResult<A, B>({
  result,
  map,
}: {
  result: ApiResult<A>;
  map: (value: A) => B;
}): ApiResult<B> {
  return result.kind === "ok" ? { ...result, value: map(result.value) } : result;
}

/** Holds a session body's token and maps it, for every call that starts a session. */
async function startSession(call: Promise<ApiResult<WireSession>>): Promise<ApiResult<Session>> {
  const result = await call;
  if (result.kind === "ok") {
    holdSession(result.value);
  }
  return mapResult({ result, map: toSession });
}

/** Swaps the refresh cookie for a session (start-up, and after `AUTH_REQUIRED`). */
export async function refresh(): Promise<ApiResult<Session>> {
  return mapResult({ result: await refreshSession<WireSession>(), map: toSession });
}

/** Starts a guest session, so a visitor can capture and design before signing up. */
export async function startGuest(): Promise<ApiResult<Session>> {
  return startSession(
    apiJson<WireSession>({ method: HttpMethod.Post, path: "/auth/guest", authenticated: false }),
  );
}

/**
 * Emails a sign-up confirmation or a sign-in link (also "forgot password"). The answer is the
 * same either way, so it never says whether an account exists.
 */
export async function requestEmailLink(email: string): Promise<ApiResult<NoContent>> {
  return apiNoContent({
    method: HttpMethod.Post,
    path: "/auth/email-link",
    body: { email },
    authenticated: false,
  });
}

/** Signs in with the token from an emailed link (confirms the address on sign-up). */
export async function signInWithLink(token: string): Promise<ApiResult<Session>> {
  return startSession(
    apiJson<WireSession>({
      method: HttpMethod.Post,
      path: "/auth/email-link/sign-in",
      body: { token },
      authenticated: false,
    }),
  );
}

/** Signs in with an email and password. */
export async function signInWithPassword({
  email,
  password,
}: {
  email: string;
  password: string;
}): Promise<ApiResult<Session>> {
  return startSession(
    apiJson<WireSession>({
      method: HttpMethod.Post,
      path: "/auth/password/sign-in",
      body: { email, password },
      authenticated: false,
    }),
  );
}

/** A passkey sign-in ceremony's options. */
export async function passkeySignInOptions(
  signal?: AbortSignal,
): Promise<ApiResult<WirePasskeySignInOptions>> {
  return apiJson<WirePasskeySignInOptions>({
    method: HttpMethod.Post,
    path: "/auth/passkey/sign-in/options",
    authenticated: false,
    signal,
  });
}

/** Signs in with the passkey the browser answered a ceremony with. */
export async function signInWithPasskey({
  ceremonyId,
  credential,
}: {
  ceremonyId: string;
  credential: AuthenticationResponse;
}): Promise<ApiResult<Session>> {
  return startSession(
    apiJson<WireSession>({
      method: HttpMethod.Post,
      path: "/auth/passkey/sign-in",
      body: { ceremony_id: ceremonyId, credential },
      authenticated: false,
    }),
  );
}

/** Ends this browser's session and clears the refresh cookie. */
export async function signOut(): Promise<ApiResult<NoContent>> {
  const result = await apiNoContent({
    method: HttpMethod.Post,
    path: "/auth/sign-out",
    authenticated: false,
  });
  dropSession();
  return result;
}

/** The signed-in account and what it has paid for. */
export async function fetchMe(signal?: AbortSignal): Promise<ApiResult<Me>> {
  const result = await apiJson<WireMe>({ path: "/me", signal });
  return mapResult({
    result,
    map: (wire) => ({
      account: toAccount(wire),
      billing: {
        plan: wire.billing.plan,
        credits: wire.billing.credits,
        unlockedDesigns: wire.billing.unlocked_designs,
        subscription: maybe(
          wire.billing.subscription === null
            ? null
            : {
                plan: wire.billing.subscription.plan,
                status: wire.billing.subscription.status,
                currentPeriodEnd: maybe(wire.billing.subscription.current_period_end),
              },
        ),
      },
    }),
  });
}

/**
 * Sets or changes the password. Within 15 minutes of a link or passkey sign-in no current
 * password is needed; otherwise the API answers `AUTH_REAUTH_REQUIRED`.
 */
export async function setPassword({
  password,
  currentPassword,
}: {
  password: string;
  currentPassword: Maybe<string>;
}): Promise<ApiResult<NoContent>> {
  return apiNoContent({
    method: HttpMethod.Put,
    path: "/me/password",
    body: {
      password,
      current_password: currentPassword.kind === "some" ? currentPassword.value : null,
    },
  });
}

/** A passkey registration ceremony's options for the signed-in account. */
export async function passkeyRegistrationOptions(): Promise<
  ApiResult<WirePasskeyRegistrationOptions>
> {
  return apiJson<WirePasskeyRegistrationOptions>({
    method: HttpMethod.Post,
    path: "/me/passkeys/options",
  });
}

/** Saves the passkey the browser made for a registration ceremony. */
export async function addPasskey({
  ceremonyId,
  credential,
  name,
}: {
  ceremonyId: string;
  credential: RegistrationResponse;
  name: string;
}): Promise<ApiResult<{ readonly id: string }>> {
  return apiJson<{ readonly id: string }>({
    method: HttpMethod.Post,
    path: "/me/passkeys",
    body: { ceremony_id: ceremonyId, credential, name },
  });
}
