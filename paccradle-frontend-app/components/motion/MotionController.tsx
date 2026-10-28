"use client";

import { usePathname } from "next/navigation";
import { useEffect } from "react";
import { none, some, type Maybe } from "@/lib/maybe";

const REVEAL_SELECTOR = "[data-reveal]:not([data-revealed])";
const GLOW_SELECTOR = "[data-glow]";
const PARALLAX_SELECTOR = "[data-parallax]";
const REDUCED_MOTION_QUERY = "(prefers-reduced-motion: reduce)";
const FINE_POINTER_QUERY = "(hover: hover) and (pointer: fine)";
/** Reveal once any part of an element is inside the top 92 % of the viewport. A ratio
 * threshold would never fire for elements much taller than the screen. */
const REVEAL_ROOT_MARGIN = "0px 0px -8% 0px";
const REVEAL_THRESHOLD = 0;
const SCROLLED_OFFSET_PX = 12;
const PARALLAX_DECIMALS = 3;

/** Tells the boot script in the root layout that the controller is running. */
const LIVE_FLAG = "motionLive";

/** A pointer position waiting for the next animation frame. */
interface PointerSample {
  readonly x: number;
  readonly y: number;
  readonly target: Element;
}

function prefersReducedMotion(): boolean {
  return window.matchMedia(REDUCED_MOTION_QUERY).matches;
}

function markRevealed(element: Element): void {
  if (element instanceof HTMLElement) {
    element.dataset.revealed = "true";
  }
}

/** Reveals `[data-reveal]` elements as they scroll into view (all at once with reduced motion). */
function observeReveals(): () => void {
  const elements = Array.from(document.querySelectorAll(REVEAL_SELECTOR));
  if (prefersReducedMotion() || !("IntersectionObserver" in window)) {
    elements.forEach(markRevealed);
    return () => {};
  }
  const observer = new IntersectionObserver(
    (entries) => {
      entries
        .filter((entry) => entry.isIntersecting)
        .forEach((entry) => {
          markRevealed(entry.target);
          observer.unobserve(entry.target);
        });
    },
    { rootMargin: REVEAL_ROOT_MARGIN, threshold: REVEAL_THRESHOLD },
  );
  elements.forEach((element) => observer.observe(element));
  return () => observer.disconnect();
}

/** Sets `data-scrolled` on <html> so the header can turn into a floating bar. */
function trackScroll(): () => void {
  const root = document.documentElement;
  let frame = 0;
  const update = (): void => {
    frame = 0;
    root.dataset.scrolled = String(window.scrollY > SCROLLED_OFFSET_PX);
  };
  const onScroll = (): void => {
    if (frame === 0) {
      frame = window.requestAnimationFrame(update);
    }
  };
  update();
  window.addEventListener("scroll", onScroll, { passive: true });
  return () => {
    window.removeEventListener("scroll", onScroll);
    window.cancelAnimationFrame(frame);
  };
}

/** Pointer position inside a box, from -1 to 1 on each axis; 0 when outside. */
function normalizedPosition(sample: PointerSample, box: DOMRect): { x: number; y: number } {
  const inside =
    sample.x >= box.left && sample.x <= box.right && sample.y >= box.top && sample.y <= box.bottom;
  if (!inside) {
    return { x: 0, y: 0 };
  }
  return {
    x: ((sample.x - box.left) / box.width) * 2 - 1,
    y: ((sample.y - box.top) / box.height) * 2 - 1,
  };
}

/**
 * Card spotlight (`--glow-x`/`--glow-y` on the hovered `[data-glow]`) and hero parallax
 * (`--parallax-x`/`--parallax-y` on `[data-parallax]`). Mouse and trackpad only.
 */
function trackPointer(): () => void {
  if (prefersReducedMotion() || !window.matchMedia(FINE_POINTER_QUERY).matches) {
    return () => {};
  }
  let frame = 0;
  let pending: Maybe<PointerSample> = none;

  const apply = (): void => {
    frame = 0;
    if (pending.kind === "none") {
      return;
    }
    const sample = pending.value;
    const card = sample.target.closest(GLOW_SELECTOR);
    if (card instanceof HTMLElement) {
      const box = card.getBoundingClientRect();
      card.style.setProperty("--glow-x", `${sample.x - box.left}px`);
      card.style.setProperty("--glow-y", `${sample.y - box.top}px`);
    }
    document.querySelectorAll<HTMLElement>(PARALLAX_SELECTOR).forEach((stage) => {
      const position = normalizedPosition(sample, stage.getBoundingClientRect());
      stage.style.setProperty("--parallax-x", position.x.toFixed(PARALLAX_DECIMALS));
      stage.style.setProperty("--parallax-y", position.y.toFixed(PARALLAX_DECIMALS));
    });
  };

  const onMove = (event: PointerEvent): void => {
    if (!(event.target instanceof Element)) {
      return;
    }
    pending = some({ x: event.clientX, y: event.clientY, target: event.target });
    if (frame === 0) {
      frame = window.requestAnimationFrame(apply);
    }
  };

  window.addEventListener("pointermove", onMove, { passive: true });
  return () => {
    window.removeEventListener("pointermove", onMove);
    window.cancelAnimationFrame(frame);
  };
}

/**
 * Client-side motion for the marketing pages: scroll reveals, the floating header,
 * card spotlights and hero parallax. Renders nothing; it only sets data attributes and
 * CSS custom properties that the stylesheets animate.
 */
export function MotionController() {
  const pathname = usePathname();

  useEffect(() => {
    document.documentElement.dataset[LIVE_FLAG] = "true";
  }, []);
  useEffect(() => observeReveals(), [pathname]);
  useEffect(() => trackScroll(), []);
  useEffect(() => trackPointer(), []);

  return null;
}
