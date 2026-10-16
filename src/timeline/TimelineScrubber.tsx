// SPDX-License-Identifier: GPL-3.0-or-later
import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent as ReactKeyboardEvent,
  type PointerEvent as ReactPointerEvent,
} from "react";
import { formatMonthYear } from "../thread/formatting";
import type { DayBucketDto } from "../lib/types";
import {
  fractionToTimestamp,
  monthTicks,
  pointerFraction,
  rangeFromDayKeys,
  stepMonths,
  timestampToFraction,
  yearTicks,
} from "./timelineMath";

/**
 * A Photos-style vertical timeline: drag anywhere on the strip to jump the
 * thread to that date, with a bold month/year bubble following your finger.
 * Position is linear in calendar time (not message density), so gaps in
 * history show as blank stretches.
 *
 * Built to stay at 60fps over a very long history:
 *  - while dragging, the handle and bubble are moved straight on the DOM
 *    (`transform`, compositor-only) and the thread is scrolled at most once
 *    per animation frame - nothing goes through React state per pointer move
 *    except the month text, and only when the month actually changes;
 *  - when not dragging, the handle glides (CSS transition) to wherever the
 *    thread has scrolled to, including the short slide home after release.
 */
export function TimelineScrubber({
  dayBuckets,
  currentTsUnixMs,
  onScrub,
  onScrubbingChange,
}: {
  dayBuckets: DayBucketDto[];
  currentTsUnixMs: number | null;
  onScrub: (tsUnixMs: number) => void;
  /** True from pointer-down to pointer-up, so the thread can go light while you scrub. */
  onScrubbingChange?: (scrubbing: boolean) => void;
}) {
  const trackRef = useRef<HTMLDivElement>(null);
  const thumbRef = useRef<HTMLDivElement>(null);
  const draggingRef = useRef(false);
  const frameRef = useRef<number | null>(null);
  const pendingYRef = useRef<number | null>(null);
  // Held in a ref so the drag's lifetime never depends on a parent re-render
  // handing us a new callback identity.
  const scrubbingChangeRef = useRef(onScrubbingChange);
  useEffect(() => {
    scrubbingChangeRef.current = onScrubbingChange;
  });

  const [trackHeight, setTrackHeight] = useState(0);
  const [dragging, setDragging] = useState(false);
  const [label, setLabel] = useState("");

  const range = useMemo(() => rangeFromDayKeys(dayBuckets.map((b) => b.day)), [dayBuckets]);
  const years = useMemo(() => (range ? yearTicks(range) : []), [range]);
  const months = useMemo(() => (range ? monthTicks(range) : []), [range]);
  const hasRange = range !== null;

  useLayoutEffect(() => {
    const track = trackRef.current;
    if (!track) return;
    const update = () => setTrackHeight(track.clientHeight);
    update();
    const observer = new ResizeObserver(update);
    observer.observe(track);
    return () => observer.disconnect();
  }, [hasRange]);

  const placeThumb = useCallback(
    (fraction: number) => {
      const thumb = thumbRef.current;
      if (thumb) thumb.style.transform = `translate3d(0, ${fraction * trackHeight}px, 0)`;
    },
    [trackHeight],
  );

  const currentFraction =
    range && currentTsUnixMs !== null ? timestampToFraction(currentTsUnixMs, range) : null;

  // Follow the thread while the user isn't dragging (wheel scroll, and the
  // glide home after a drag ends). While dragging the pointer owns the handle.
  useLayoutEffect(() => {
    if (!draggingRef.current && currentFraction !== null) placeThumb(currentFraction);
  }, [currentFraction, placeThumb, dragging]);

  const flush = useCallback(() => {
    frameRef.current = null;
    const y = pendingYRef.current;
    const track = trackRef.current;
    if (y === null || !track || !range) return;
    pendingYRef.current = null;

    const rect = track.getBoundingClientRect();
    const fraction = pointerFraction(y, rect.top, rect.height);
    const tsUnixMs = fractionToTimestamp(fraction, range);

    placeThumb(fraction);
    // React bails out when the text is unchanged, so this only renders on a
    // month boundary.
    setLabel(formatMonthYear(tsUnixMs));
    onScrub(tsUnixMs);
  }, [onScrub, placeThumb, range]);

  const queueScrub = useCallback(
    (clientY: number) => {
      pendingYRef.current = clientY;
      if (frameRef.current === null) frameRef.current = requestAnimationFrame(flush);
    },
    [flush],
  );

  const endDrag = useCallback(() => {
    if (!draggingRef.current) return;
    draggingRef.current = false;
    if (frameRef.current !== null) {
      cancelAnimationFrame(frameRef.current);
      frameRef.current = null;
    }
    pendingYRef.current = null;
    setDragging(false);
    scrubbingChangeRef.current?.(false);
  }, []);

  // Never leave the thread stuck in "scrubbing" mode if this unmounts mid-drag.
  useEffect(() => () => endDrag(), [endDrag]);

  if (!range) return null;

  const onPointerDown = (event: ReactPointerEvent<HTMLDivElement>) => {
    if (event.button !== 0) return;
    try {
      event.currentTarget.setPointerCapture(event.pointerId);
    } catch {
      // The pointer is already gone (or synthetic); dragging still works
      // through the move events that follow.
    }
    draggingRef.current = true;
    setDragging(true);
    scrubbingChangeRef.current?.(true);
    queueScrub(event.clientY);
  };

  const onPointerMove = (event: ReactPointerEvent<HTMLDivElement>) => {
    if (draggingRef.current) queueScrub(event.clientY);
  };

  const onKeyDown = (event: ReactKeyboardEvent<HTMLDivElement>) => {
    const step =
      event.key === "ArrowUp" ? -1
      : event.key === "ArrowDown" ? 1
      : event.key === "PageUp" ? -12
      : event.key === "PageDown" ? 12
      : 0;
    if (step === 0 || currentTsUnixMs === null) return;
    event.preventDefault();
    onScrub(stepMonths(currentTsUnixMs, step));
  };

  return (
    <div className={`timeline-scrubber${dragging ? " is-dragging" : ""}`}>
      <div
        className="timeline-track"
        ref={trackRef}
        role="slider"
        tabIndex={0}
        aria-label="Timeline"
        aria-orientation="vertical"
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={Math.round((currentFraction ?? 0) * 100)}
        aria-valuetext={currentTsUnixMs !== null ? formatMonthYear(currentTsUnixMs) : undefined}
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={endDrag}
        onPointerCancel={endDrag}
        onLostPointerCapture={endDrag}
        onKeyDown={onKeyDown}
      >
        {months.map((tick) => (
          <div
            key={tick.tsUnixMs}
            className="timeline-tick-month"
            style={{ top: `${tick.fraction * 100}%` }}
          />
        ))}
        {years.map((tick) => (
          <div
            key={tick.tsUnixMs}
            className="timeline-tick-year"
            style={{ top: `${tick.fraction * 100}%` }}
          >
            <span className="timeline-tick-year-label">{tick.label}</span>
          </div>
        ))}

        <div className="timeline-thumb" ref={thumbRef}>
          <div className="timeline-label" aria-hidden="true">
            {label}
          </div>
          <div className="timeline-handle" />
        </div>
      </div>
    </div>
  );
}
