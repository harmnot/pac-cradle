"use client";

import { createContext, use, useCallback, useEffect, useMemo, useReducer, type ReactNode } from "react";
import { ClientErrorCode, onSessionChange, type ApiError, type ApiResult } from "@/lib/api-client";
import { none, some, type Maybe } from "@/lib/maybe";
import { clearQueryCache } from "@/lib/use-api";
import * as identity from "./api";
import { AccountRole, type Account, type NextStep, type Session, type WireSession } from "./types";

/**
 * The session in React (DEC-05: `useReducer` + Context): who is signed in, what the API asked
 * them to do next, and whether the API could be reached. The access token itself stays in
 * `lib/api-client`. Tabs keep each other in step over a `BroadcastChannel`.
 */

/** Where the session stands. */
export type SessionState =
  /** Asking the API whether the refresh cookie still holds a session. */
  | { readonly status: "starting" }
  /** No session: a visitor. A guest session starts at the first action that needs one. */
  | { readonly status: "none" }
  /** The API couldn't be reached to find out. */
  | { readonly status: "unreachable"; readonly error: ApiError }
  | { readonly status: "active"; readonly account: Account; readonly nextSteps: readonly NextStep[] };

type Action =
  | { readonly type: "started"; readonly session: Session }
  | { readonly type: "ended" }
  | { readonly type: "unreachable"; readonly error: ApiError }
  | { readonly type: "checking" }
  | { readonly type: "account"; readonly account: Account }
  | { readonly type: "step_done"; readonly step: NextStep };

function reduce(state: SessionState, action: Action): SessionState {
  switch (action.type) {
    case "started":
      return { status: "active", account: action.session.account, nextSteps: action.session.nextSteps };
    case "ended":
      return { status: "none" };
    case "unreachable":
      return { status: "unreachable", error: action.error };
    case "checking":
      return { status: "starting" };
    case "account":
      return state.status === "active" ? { ...state, account: action.account } : state;
    case "step_done":
      return state.status === "active"
        ? { ...state, nextSteps: state.nextSteps.filter((step) => step !== action.step) }
        : state;
  }
}

/** What components get from the session. */
export interface SessionApi {
  readonly state: SessionState;
  /** Takes a session a sign-in screen just started. */
  readonly signedIn: (session: Session) => void;
  /** Ends the session here and in the other tabs. */
  readonly signOut: () => Promise<void>;
  /** The current session, starting a guest one when there is none (capture, design). */
  readonly ensureSession: () => Promise<ApiResult<Session>>;
  /** Marks one of the sign-in's next steps as done (or declined). */
  readonly completeStep: (step: NextStep) => void;
  /** Reads the account again (after a password or passkey change). */
  readonly reloadAccount: () => Promise<void>;
  /** Asks the API again after it couldn't be reached. */
  readonly retry: () => void;
}

const SessionContext = createContext<SessionApi | null>(null);

const CHANNEL = "paccradle-session";
const SESSION_CHANGED = "changed";

function openChannel(): Maybe<BroadcastChannel> {
  return typeof BroadcastChannel === "function" ? some(new BroadcastChannel(CHANNEL)) : none;
}

function tellOtherTabs(): void {
  const channel = openChannel();
  if (channel.kind === "some") {
    channel.value.postMessage(SESSION_CHANGED);
    channel.value.close();
  }
}

/** Whether a failed refresh means "no session" rather than "can't tell right now". */
function meansNoSession(error: ApiError): boolean {
  return error.status >= 400 && error.status < 500;
}

/** Provides the session to the app's pages. */
export function SessionProvider({ children }: { readonly children: ReactNode }) {
  const [state, dispatch] = useReducer(reduce, { status: "starting" });

  const check = useCallback(async () => {
    const result = await identity.refresh();
    if (result.kind === "ok") {
      dispatch({ type: "started", session: result.value });
      return;
    }
    if (result.error.code === ClientErrorCode.Aborted) {
      return;
    }
    dispatch(meansNoSession(result.error) ? { type: "ended" } : { type: "unreachable", error: result.error });
  }, []);

  useEffect(() => {
    void check();
    // A refresh made by any call (after AUTH_REQUIRED) can change the account or end it.
    const unsubscribe = onSessionChange((event) => {
      if (event.kind === "expired") {
        clearQueryCache();
        dispatch({ type: "ended" });
        return;
      }
      dispatch({ type: "started", session: identity.toSession(event.body as WireSession) });
    });
    const channel = openChannel();
    const onMessage = () => {
      clearQueryCache();
      void check();
    };
    if (channel.kind === "some") {
      channel.value.addEventListener("message", onMessage);
    }
    return () => {
      unsubscribe();
      if (channel.kind === "some") {
        channel.value.removeEventListener("message", onMessage);
        channel.value.close();
      }
    };
  }, [check]);

  const signedIn = useCallback((session: Session) => {
    clearQueryCache();
    dispatch({ type: "started", session });
    tellOtherTabs();
  }, []);

  const signOut = useCallback(async () => {
    await identity.signOut();
    clearQueryCache();
    dispatch({ type: "ended" });
    tellOtherTabs();
  }, []);

  const ensureSession = useCallback(async (): Promise<ApiResult<Session>> => {
    if (state.status === "active") {
      return { kind: "ok", status: 200, value: { account: state.account, nextSteps: state.nextSteps } };
    }
    const result = await identity.startGuest();
    if (result.kind === "ok") {
      dispatch({ type: "started", session: result.value });
    }
    return result;
  }, [state]);

  const completeStep = useCallback((step: NextStep) => dispatch({ type: "step_done", step }), []);

  const reloadAccount = useCallback(async () => {
    const result = await identity.fetchMe();
    if (result.kind === "ok") {
      dispatch({ type: "account", account: result.value.account });
    }
  }, []);

  const retry = useCallback(() => {
    dispatch({ type: "checking" });
    void check();
  }, [check]);

  const api = useMemo<SessionApi>(
    () => ({ state, signedIn, signOut, ensureSession, completeStep, reloadAccount, retry }),
    [state, signedIn, signOut, ensureSession, completeStep, reloadAccount, retry],
  );
  return <SessionContext value={api}>{children}</SessionContext>;
}

/** The session, inside `SessionProvider`. */
export function useSession(): SessionApi {
  const api = use(SessionContext);
  if (api === null) {
    throw new Error("useSession must be used inside SessionProvider");
  }
  return api;
}

/** Whether the session belongs to a confirmed account (not a guest). */
export function isAccount(state: SessionState): state is Extract<SessionState, { status: "active" }> {
  return state.status === "active" && state.account.role !== AccountRole.Guest;
}
