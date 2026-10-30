import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  apiJson,
  apiNoContent,
  dropSession,
  hasSession,
  holdSession,
  HttpMethod,
  onSessionChange,
  type SessionEvent,
} from "./api-client";

/** One answer the pretend API gives: a status and an optional JSON body. */
interface Answer {
  readonly status: number;
  readonly body?: unknown;
}

/** A call the client made: path under the client prefix, method and headers. */
interface Sent {
  readonly path: string;
  readonly method: string;
  readonly authorization: string;
  readonly credentials: RequestCredentials | undefined;
}

const PREFIX = "http://localhost:7773/api/client/v1";

function authRequired(): Answer {
  return {
    status: 401,
    body: { error: { code: "AUTH_REQUIRED", message: "Please sign in.", meta: {} } },
  };
}

/**
 * A pretend API: each path answers from its own queue, in order, and every call is recorded.
 * An `Error` in a queue is thrown as a network failure.
 */
function pretendApi(routes: Record<string, Array<Answer | Error>>): Sent[] {
  const sent: Sent[] = [];
  vi.stubGlobal(
    "fetch",
    vi.fn(async (url: string, init: RequestInit) => {
      const path = url.slice(PREFIX.length);
      const headers = (init.headers ?? {}) as Record<string, string>;
      sent.push({
        path,
        method: init.method ?? "GET",
        authorization: headers.Authorization ?? "",
        credentials: init.credentials,
      });
      const next = routes[path]?.shift();
      if (next === undefined) {
        throw new Error(`no answer left for ${path}`);
      }
      if (next instanceof Error) {
        throw new TypeError("Failed to fetch");
      }
      const body = next.body === undefined ? null : JSON.stringify(next.body);
      return new Response(body, { status: next.status });
    }),
  );
  return sent;
}

beforeEach(() => {
  dropSession();
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.useRealTimers();
});

describe("the access token", () => {
  it("rides on every call, with the cookies", async () => {
    holdSession({ access_token: "first" });
    const sent = pretendApi({ "/me": [{ status: 200, body: { id: "ada" } }] });

    const result = await apiJson<{ id: string }>({ path: "/me" });

    expect(result).toEqual({ kind: "ok", value: { id: "ada" }, status: 200 });
    expect(sent).toEqual([
      { path: "/me", method: "GET", authorization: "Bearer first", credentials: "include" },
    ]);
  });

  it("is refreshed once on AUTH_REQUIRED, and the call is sent again with the new one", async () => {
    holdSession({ access_token: "stale" });
    const sent = pretendApi({
      "/me": [authRequired(), { status: 200, body: { id: "ada" } }],
      "/auth/refresh": [{ status: 200, body: { access_token: "fresh" } }],
    });
    const events: SessionEvent[] = [];
    const stop = onSessionChange((event) => events.push(event));

    const result = await apiJson<{ id: string }>({ path: "/me" });
    stop();

    expect(result.kind).toBe("ok");
    expect(sent.map((call) => `${call.method} ${call.path} ${call.authorization}`)).toEqual([
      "GET /me Bearer stale",
      "POST /auth/refresh ",
      "GET /me Bearer fresh",
    ]);
    expect(events).toEqual([{ kind: "refreshed", body: { access_token: "fresh" } }]);
  });

  it("is refreshed once for calls that fail together", async () => {
    holdSession({ access_token: "stale" });
    const sent = pretendApi({
      "/me": [authRequired(), { status: 200, body: { id: "ada" } }],
      "/designs": [authRequired(), { status: 200, body: { items: [] } }],
      "/auth/refresh": [{ status: 200, body: { access_token: "fresh" } }],
    });

    const [me, designs] = await Promise.all([
      apiJson({ path: "/me" }),
      apiJson({ path: "/designs" }),
    ]);

    expect(me.kind).toBe("ok");
    expect(designs.kind).toBe("ok");
    expect(sent.filter((call) => call.path === "/auth/refresh")).toHaveLength(1);
  });

  it("is dropped when the session has ended, and the caller hears AUTH_SESSION_EXPIRED", async () => {
    holdSession({ access_token: "stale" });
    pretendApi({
      "/me": [authRequired()],
      "/auth/refresh": [
        {
          status: 401,
          body: { error: { code: "AUTH_SESSION_EXPIRED", message: "Ended.", meta: {} } },
        },
      ],
    });
    const events: SessionEvent[] = [];
    const stop = onSessionChange((event) => events.push(event));

    const result = await apiJson({ path: "/me" });
    stop();

    expect(result.kind === "error" && result.error.code).toBe("AUTH_SESSION_EXPIRED");
    expect(hasSession()).toBe(false);
    expect(events).toEqual([{ kind: "expired" }]);
  });

  it("is never refreshed for calls that don't use it", async () => {
    const sent = pretendApi({
      "/auth/password/sign-in": [
        {
          status: 401,
          body: { error: { code: "AUTH_REQUIRED", message: "", meta: {} } },
        },
      ],
    });

    await apiJson({
      method: HttpMethod.Post,
      path: "/auth/password/sign-in",
      body: {},
      authenticated: false,
    });

    expect(sent).toHaveLength(1);
    expect(sent[0].authorization).toBe("");
  });
});

describe("failures", () => {
  it("retry a read after a 503, with a pause", async () => {
    vi.useFakeTimers();
    const sent = pretendApi({
      "/designs": [{ status: 503 }, { status: 200, body: { items: [] } }],
    });

    const pending = apiJson({ path: "/designs" });
    await vi.advanceTimersByTimeAsync(400);
    const result = await pending;

    expect(result.kind).toBe("ok");
    expect(sent).toHaveLength(2);
  });

  it("never retry a write: its error comes back as the API sent it", async () => {
    const sent = pretendApi({
      "/designs": [
        {
          status: 503,
          body: { error: { code: "SERVICE_UNAVAILABLE", message: "Down.", meta: {} } },
        },
      ],
    });

    const result = await apiJson({ method: HttpMethod.Post, path: "/designs", body: {} });

    expect(sent).toHaveLength(1);
    expect(result).toEqual({
      kind: "error",
      error: { status: 503, code: "SERVICE_UNAVAILABLE", message: "Down.", meta: {} },
    });
  });

  it("with no answer at all become NETWORK_ERROR once the retries run out", async () => {
    vi.useFakeTimers();
    const sent = pretendApi({ "/me": [new Error(), new Error(), new Error()] });

    const pending = apiJson({ path: "/me" });
    await vi.advanceTimersByTimeAsync(2000);
    const result = await pending;

    expect(sent).toHaveLength(3);
    expect(result.kind === "error" && result.error.code).toBe("NETWORK_ERROR");
  });

  it("answer ABORTED when the caller gave up", async () => {
    pretendApi({ "/me": [] });
    const controller = new AbortController();
    controller.abort();

    const result = await apiJson({ path: "/me", signal: controller.signal });

    expect(result.kind === "error" && result.error.code).toBe("ABORTED");
  });

  it("that aren't the API's error shape become BAD_RESPONSE", async () => {
    pretendApi({ "/me": [{ status: 500, body: "<html>oops</html>" }] });

    const result = await apiJson({ method: HttpMethod.Post, path: "/me" });

    expect(result.kind === "error" && result.error).toEqual({
      status: 500,
      code: "BAD_RESPONSE",
      message: "",
      meta: {},
    });
  });
});

describe("uploads", () => {
  it("send FormData as it is, letting the browser set the multipart type", async () => {
    let sentBody: unknown = null;
    let sentHeaders: Record<string, string> = {};
    vi.stubGlobal(
      "fetch",
      vi.fn(async (_url: string, init: RequestInit) => {
        sentBody = init.body;
        sentHeaders = init.headers as Record<string, string>;
        return new Response(JSON.stringify({ capture_id: "c1" }), { status: 201 });
      }),
    );
    const form = new FormData();
    form.append("capture", JSON.stringify({ paper: "a4", mode: "photo" }));

    const result = await apiJson({ method: HttpMethod.Post, path: "/captures", body: form });

    expect(result.kind).toBe("ok");
    expect(sentBody).toBe(form);
    expect(sentHeaders["Content-Type"]).toBeUndefined();
  });
});

describe("empty answers", () => {
  it("read as no_content", async () => {
    pretendApi({ "/auth/email-link": [{ status: 202 }] });

    const result = await apiNoContent({
      method: HttpMethod.Post,
      path: "/auth/email-link",
      body: { email: "buyer@paccradle.test" },
      authenticated: false,
    });

    expect(result).toEqual({ kind: "ok", value: "no_content", status: 202 });
  });
});
