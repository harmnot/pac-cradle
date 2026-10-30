import { ViewTransition, type ReactNode } from "react";

/**
 * Transition types for links and `router.push` (`transitionTypes`), matched by
 * styles/transitions.css.
 */
export const TransitionType = {
  /** Deeper into the app: content slides left. */
  Forward: "nav-forward",
  /** Back out: content slides right. */
  Back: "nav-back",
} as const;

export type TransitionType = (typeof TransitionType)[keyof typeof TransitionType];

const DIRECTIONAL = {
  [TransitionType.Forward]: TransitionType.Forward,
  [TransitionType.Back]: TransitionType.Back,
  default: "none",
} as const;

/**
 * A page's content, sliding in the direction of the navigation that brought it. Put it in each
 * `page.tsx`, not a layout: layouts stay mounted, so their content never enters or leaves.
 * Navigations without a type (the browser's back button) just swap.
 */
export function PageTransition({ children }: { readonly children: ReactNode }) {
  return (
    <ViewTransition enter={DIRECTIONAL} exit={DIRECTIONAL} default="none">
      {children}
    </ViewTransition>
  );
}

/**
 * One screen's content changing in place (sign-in: passkey → password → inbox). Changing
 * `step` inside `startTransition` lifts the old content out and the new one in.
 */
export function StepTransition({ step, children }: { readonly step: string; readonly children: ReactNode }) {
  return (
    <ViewTransition key={step} enter="step" exit="step" default="none">
      {children}
    </ViewTransition>
  );
}
