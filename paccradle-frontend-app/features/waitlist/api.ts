import { publicConfig } from "@/lib/config";
import type { WaitlistRequest, WaitlistResult } from "./types";

/**
 * Sends a sign-up to the hosted email-list tool (M0-04: no backend needed yet).
 * The endpoint receives a form POST with `email` and `source` fields.
 */
export async function joinWaitlist(request: WaitlistRequest): Promise<WaitlistResult> {
  const endpoint = publicConfig.waitlistEndpoint;
  if (endpoint.kind === "none") {
    return { kind: "error", reason: "not_configured" };
  }
  try {
    const response = await fetch(endpoint.value, {
      method: "POST",
      headers: { Accept: "application/json" },
      body: new URLSearchParams({ email: request.email, source: request.source }),
    });
    return response.ok ? { kind: "ok" } : { kind: "error", reason: "rejected" };
  } catch {
    return { kind: "error", reason: "network" };
  }
}
