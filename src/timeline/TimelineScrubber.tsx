// SPDX-License-Identifier: GPL-3.0-or-later
import { useMemo, useRef, useState, type PointerEvent as ReactPointerEvent } from "react";
import { formatMonthYear } from "../thread/formatting";
import type { DayBucketDto } from "../lib/types";
import {
  fractionToTimestamp,
  monthTicks,
  rangeFromDayKeys,
  timestampToFraction,
  yearTicks,
} from "./timelineMath";

/**
 * A Photos-style vertical timeline: drag anywhere on the strip to jump the
 * thread to that date. Position is linear in calendar time (not message
 * density), so gaps in history show as blank stretches - the point is "map
 * scroll position to date," not "spend equal pixels per message."
 */
export function TimelineScrubber({
  dayBuckets,
  currentTsUnixMs,
  onScrub,
}: {
  dayBuckets: DayBucketDto[];
  currentTsUnixMs: number | null;
  onScrub: (tsUnixMs: number) => void;
}) {
  const trackRef = useRef<HTMLDivElement>(null);
  const [drag, setDrag] = useState<{ fraction: number; label: string } | null>(null);

  const range = useMemo(() => rangeFromDayKeys(dayBuckets.map((b) => b.day)), [dayBuckets]);
  const years = useMemo(() => (range ? yearTicks(range) : []), [range]);
  const months = useMemo(() => (range ? monthTicks(range) : []), [range]);

  if (!range) return null;

  const scrubAt = (clientY: number) => {
    const track = trackRef.current;
    if (!track) return;
    const rect = track.getBoundingClientRect();
    const fraction = rect.height > 0 ? (clientY - rect.top) / rect.height : 0;
    const tsUnixMs = fractionToTimestamp(fraction, range);
    setDrag({ fraction: Math.min(1, Math.max(0, fraction)), label: formatMonthYear(tsUnixMs) });
    onScrub(tsUnixMs);
  };

  const onPointerDown = (e: ReactPointerEvent<HTMLDivElement>) => {
    e.currentTarget.setPointerCapture(e.pointerId);
    scrubAt(e.clientY);
  };
  const onPointerMove = (e: ReactPointerEvent<HTMLDivElement>) => {
    if (e.buttons !== 1) return;
    scrubAt(e.clientY);
  };
  const onPointerUp = () => setDrag(null);

  const currentFraction =
    currentTsUnixMs !== null ? timestampToFraction(currentTsUnixMs, range) : null;

  return (
    <div className="timeline-scrubber">
      <div
        className="timeline-track"
        ref={trackRef}
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerUp}
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
        {currentFraction !== null && (
          <div className="timeline-current" style={{ top: `${currentFraction * 100}%` }} />
        )}
      </div>

      {drag && (
        <div className="timeline-drag-label" style={{ top: `${drag.fraction * 100}%` }}>
          {drag.label}
        </div>
      )}
    </div>
  );
}
