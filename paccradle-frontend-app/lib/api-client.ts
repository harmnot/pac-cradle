import { publicConfig } from "./config";
import { none, some, type Maybe } from "./maybe";

/**
 * The typed fetch wrapper for the client API (`/api/client/v1`, MILESTONES M1-E1):
 *
 * - The access token lives in this module's memory only, never in storage or React state.
 * - Every call sends cookies (`credentials: "include"`), so the refresh cookie travels.
 * - On `AUTH_REQUIRED` it refreshes once (`POST /auth/refresh`) and resends. Refreshes are
 *   shared within a tab and serialised across tabs (`navigator.locks`), so two tabs never
 *   race to swap the same cookie.
 * - Reads (`GET`) are retried on network errors and 502/503/504, with a short back-off.
 * - Callers pass an `AbortSignal`; an aborted call answers `ABORTED`.
 */

/** HTTP methods the API uses. */
export const HttpMethod = {
  Get: "GET",
  Post: "POST",
  Put: "PUT",
  Patch: "PATCH",
  Delete: "DELETE",
} as const;

export type HttpMethod = (typeof HttpMethod)[keyof typeof HttpMethod];

/** Error codes the client itself produces (the API's own come in its error body). */
export const ClientErrorCode = {
  /** No answer: offline, DNS, CORS, the API down. */
  Network: "NETWORK_ERROR",
  /** The caller aborted the call. */
  Aborted: "ABORTED",
  /** The answer wasn't the JSON it should be. */
  BadResponse: "BAD_RESPONSE",
} as const;

/** API error codes the client acts on (`backend-architecture.md` §4.1). */
export const ApiErrorCode = {
  AuthRequired: "AUTH_REQUIRED",
  SessionExpired: "AUTH_SESSION_EXPIRED",
  RateLimited: "RATE_LIMITED",
} as const;

/** A failed call: the API's error body, or one of [`ClientErrorCode`]. */
export interface ApiError {
  /** HTTP status; 0 when there was no answer. */
  readonly status: number;
  readonly code: string;
  /** The API's message, already in the caller's language; empty for client errors. */
  readonly message: string;
  readonly meta: Readonly<Record<string, unknown>>;
}

/** The outcome of a call. */
export type ApiResult<T> =
  | { readonly kind: "ok"; readonly value: T; readonly status: number }
  | { readonly kind: "error"; readonly error: ApiError };

/** What a call with an empty answer (202, 204) gives back. */
export const NO_CONTENT = "no_content";
export type NoContent = typeof NO_CONTENT;

/** How to read an answer's body. */
export const Expect = {
  Json: "json",
  Nothing: "nothing",
} as const;

export type Expect = (typeof Expect)[keyof typeof Expect];

/** A call to the client API. */
export interface ApiRequest {
  readonly method?: HttpMethod;
  /** Path under `/api/client/v1`, starting with `/`, query included. */
  readonly path: string;
  /** JSON body, or `FormData` for a multipart upload (the browser sets its boundary). */
  readonly body?: unknown;
  readonly signal?: AbortSignal;
  /** Send the access token and refresh on `AUTH_REQUIRED` (default true). */
  readonly authenticated?: boolean;
}

/** A session body's token part: every sign-in and refresh answers with one. */
interface TokenBody {
  readonly access_token: string;
}

/** Something that wants to know when a refresh changed the session. */
export type SessionListener = (event: SessionEvent) => void;

/** What happened to the session outside the caller's own call. */
export type SessionEvent =
  | { readonly kind: "refreshed"; readonly body: unknown }
  | { readonly kind: "expired" };

const CLIENT_PREFIX = "/api/client/v1";
const REFRESH_PATH = "/auth/refresh";
const REFRESH_LOCK = "paccradle-refresh";
const LANGUAGE = "en";
/** Waits before each retry of a read. */
const RETRY_DELAYS_MS = [400, 1200] as const;
const RETRIED_STATUSES: ReadonlySet<number> = new Set([502, 503, 504]);

let accessToken: Maybe<string> = none;
let refreshing: Maybe<Promise<ApiResult<unknown>>> = none;
const listeners = new Set<SessionListener>();

/** Keeps the access token of a session body the caller just received. */
export function holdSession(body: TokenBody): void {
  accessToken = some(body.access_token);
}

/** Forgets the access token (sign-out). */
export function dropSession(): void {
  accessToken = none;
}

/** Whether a token is held: a session has started in this tab. */
export function hasSession(): boolean {
  return accessToken.kind === "some";
}

/** Calls `listener` whenever a refresh changes the session; returns the unsubscribe. */
export function onSessionChange(listener: SessionListener): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

function emit(event: SessionEvent): void {
  listeners.forEach((listener) => listener(event));
}

function clientError({ code, status = 0 }: { code: string; status?: number }): ApiError {
  return { status, code, message: "", meta: {} };
}

function isAbort(error: unknown): boolean {
  return error instanceof DOMException && error.name === "AbortError";
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

/** The API's `{ error: { code, message, meta } }`, or a client error when it isn't one. */
async function readError(response: Response): Promise<ApiError> {
  const body: unknown = await response.json().catch(() => none);
  const error = isRecord(body) ? body.error : none;
  if (!isRecord(error) || typeof error.code !== "string") {
    return clientError({ code: ClientErrorCode.BadResponse, status: response.status });
  }
  return {
    status: response.status,
    code: error.code,
    message: typeof error.message === "string" ? error.message : "",
    meta: isRecord(error.meta) ? error.meta : {},
  };
}

async function readBody<T>({
  response,
  expect,
}: {
  response: Response;
  expect: Expect;
}): Promise<ApiResult<T>> {
  if (!response.ok) {
    return { kind: "error", error: await readError(response) };
  }
  if (expect === Expect.Nothing) {
    return { kind: "ok", value: NO_CONTENT as T, status: response.status };
  }
  try {
    const value = (await response.json()) as T;
    return { kind: "ok", value, status: response.status };
  } catch {
    return {
      kind: "error",
      error: clientError({ code: ClientErrorCode.BadResponse, status: response.status }),
    };
  }
}

/** Waits `ms`, or less when `signal` aborts. */
async function pause({ ms, signal }: { ms: number; signal?: AbortSignal }): Promise<void> {
  await new Promise<void>((resolve) => {
    const timer = setTimeout(resolve, ms);
    signal?.addEventListener(
      "abort",
      () => {
        clearTimeout(timer);
        resolve();
      },
      { once: true },
    );
  });
}

/** One `fetch` of `request`, with retries for reads. */
async function send<T>({
  request,
  expect,
}: {
  request: ApiRequest;
  expect: Expect;
}): Promise<ApiResult<T>> {
  const method = request.method ?? HttpMethod.Get;
  const headers: Record<string, string> = {
    Accept: "application/json",
    "Accept-Language": LANGUAGE,
  };
  const multipart = typeof FormData !== "undefined" && request.body instanceof FormData;
  const body: BodyInit | undefined = multipart
    ? (request.body as FormData)
    : request.body === undefined
      ? undefined
      : JSON.stringify(request.body);
  if (request.body !== undefined && !multipart) {
    headers["Content-Type"] = "application/json";
  }
  if ((request.authenticated ?? true) && accessToken.kind === "some") {
    headers.Authorization = `Bearer ${accessToken.value}`;
  }
  const waits = method === HttpMethod.Get ? [0, ...RETRY_DELAYS_MS] : [0];
  for (const [attempt, wait] of waits.entries()) {
    if (wait > 0) {
      await pause({ ms: wait, signal: request.signal });
    }
    if (request.signal?.aborted) {
      return { kind: "error", error: clientError({ code: ClientErrorCode.Aborted }) };
    }
    try {
      const response = await fetch(`${publicConfig.apiUrl}${CLIENT_PREFIX}${request.path}`, {
        method,
        headers,
        body,
        credentials: "include",
        cache: "no-store",
        signal: request.signal,
      });
      const lastAttempt = attempt === waits.length - 1;
      if (lastAttempt || !RETRIED_STATUSES.has(response.status)) {
        return await readBody<T>({ response, expect });
      }
    } catch (error) {
      if (isAbort(error)) {
        return { kind: "error", error: clientError({ code: ClientErrorCode.Aborted }) };
      }
    }
  }
  return { kind: "error", error: clientError({ code: ClientErrorCode.Network }) };
}

/** Runs `task` while holding the cross-tab refresh lock, where the browser has one. */
async function withRefreshLock<T>(task: () => Promise<T>): Promise<T> {
  if (typeof navigator !== "undefined" && "locks" in navigator) {
    return navigator.locks.request(REFRESH_LOCK, task);
  }
  return task();
}

/**
 * Swaps the refresh cookie for a new session, once per tab at a time. Holds the new token
 * and tells the listeners; an ended session (`AUTH_SESSION_EXPIRED`) drops the token.
 */
export async function refreshSession<T extends TokenBody>(): Promise<ApiResult<T>> {
  if (refreshing.kind === "some") {
    return (await refreshing.value) as ApiResult<T>;
  }
  const run = withRefreshLock(() =>
    send<T>({
      request: { method: HttpMethod.Post, path: REFRESH_PATH, authenticated: false },
      expect: Expect.Json,
    }),
  );
  refreshing = some(run);
  try {
    const result = await run;
    if (result.kind === "ok") {
      holdSession(result.value);
      emit({ kind: "refreshed", body: result.value });
    } else if (result.error.code === ApiErrorCode.SessionExpired) {
      dropSession();
      emit({ kind: "expired" });
    }
    return result;
  } finally {
    refreshing = none;
  }
}

async function call<T>({
  request,
  expect,
}: {
  request: ApiRequest;
  expect: Expect;
}): Promise<ApiResult<T>> {
  const first = await send<T>({ request, expect });
  const mayRefresh =
    (request.authenticated ?? true) &&
    first.kind === "error" &&
    first.error.code === ApiErrorCode.AuthRequired;
  if (!mayRefresh) {
    return first;
  }
  const refreshed = await refreshSession();
  if (refreshed.kind === "error") {
    return refreshed;
  }
  return send<T>({ request, expect });
}

/** Calls the client API and reads a JSON answer. */
export async function apiJson<T>(request: ApiRequest): Promise<ApiResult<T>> {
  return call<T>({ request, expect: Expect.Json });
}

/** Calls the client API where the answer has no body (202, 204). */
export async function apiNoContent(request: ApiRequest): Promise<ApiResult<NoContent>> {
  return call<NoContent>({ request, expect: Expect.Nothing });
}
