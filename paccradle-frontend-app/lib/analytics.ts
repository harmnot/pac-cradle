/**
 * Plausible custom events (DEC-05). Page views are counted by the script itself.
 * The fake-door rate for Gate 0 is fake_door_click ÷ unique visitors.
 */
export const AnalyticsEvent = {
  FakeDoorClick: "fake_door_click",
  WaitlistSignup: "waitlist_signup",
} as const;

export type AnalyticsEvent = (typeof AnalyticsEvent)[keyof typeof AnalyticsEvent];

/** Properties attached to an event (Plausible only accepts string values here). */
export type EventProps = Readonly<Record<string, string>>;

declare global {
  interface Window {
    plausible?: (event: string, options?: { props?: Record<string, string> }) => void;
  }
}

/**
 * Sends a custom event. Does nothing when the script isn't loaded
 * (analytics not configured, or blocked by the visitor).
 */
export function trackEvent(event: AnalyticsEvent, props: EventProps = {}): void {
  window.plausible?.(event, { props: { ...props } });
}
