// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * How quickly the thread catches up with where the timeline points, as the
 * time constant of an exponential ease: roughly 63% of the remaining
 * distance is covered every `SCROLL_TAU_MS`. Short enough to feel attached
 * to your finger, long enough that a jump across years glides instead of
 * snapping.
 */
export const SCROLL_TAU_MS = 95;

/** Closer than this (in px) counts as arrived. */
export const SCROLL_SNAP_PX = 0.75;

/**
 * One frame of an exponential ease toward `target`. Framerate independent:
 * two 8ms steps land exactly where one 16ms step does, so the glide looks
 * the same on a 60Hz and a 144Hz display (and doesn't speed up or slow down
 * when a frame is dropped). Never overshoots.
 */
export function easeToward(current: number, target: number, dtMs: number, tauMs = SCROLL_TAU_MS): number {
  const remaining = target - current;
  if (Math.abs(remaining) <= SCROLL_SNAP_PX) return target;
  const fraction = 1 - Math.exp(-Math.max(0, dtMs) / tauMs);
  const next = current + remaining * fraction;
  // Never stall just short of the target because the step rounded to nothing.
  return Math.abs(target - next) <= SCROLL_SNAP_PX ? target : next;
}

/** Whether the user has asked their system to cut down on motion. */
export function prefersReducedMotion(): boolean {
  return typeof window !== "undefined" && window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}
