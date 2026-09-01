// SPDX-License-Identifier: GPL-3.0-or-later
//! Pure position <-> date math for the timeline scrubber. Kept separate
//! from the component so it's cheap to unit test without a DOM.

export interface TimelineRange {
  minTsUnixMs: number;
  maxTsUnixMs: number;
}

/** `day_index` rows are "YYYY-MM-DD"; parsed as UTC midnight (matches how the backend buckets them - see day_bucket() in amber-format). */
export function parseDayKey(day: string): number {
  const [year, month, date] = day.split("-").map(Number);
  return Date.UTC(year, month - 1, date);
}

export function rangeFromDayKeys(days: string[]): TimelineRange | null {
  if (days.length === 0) return null;
  let min = Infinity;
  let max = -Infinity;
  for (const day of days) {
    const ts = parseDayKey(day);
    if (ts < min) min = ts;
    if (ts > max) max = ts;
  }
  // Extend to the end of the last day so the range has non-zero span even
  // for a single-day conversation.
  return { minTsUnixMs: min, maxTsUnixMs: max + 24 * 60 * 60 * 1000 };
}

/** Maps a scrubber position (0 = top/oldest, 1 = bottom/newest) to a timestamp, linear in calendar time. */
export function fractionToTimestamp(fraction: number, range: TimelineRange): number {
  const clamped = Math.min(1, Math.max(0, fraction));
  return range.minTsUnixMs + clamped * (range.maxTsUnixMs - range.minTsUnixMs);
}

/** Inverse of {@link fractionToTimestamp}. */
export function timestampToFraction(tsUnixMs: number, range: TimelineRange): number {
  const span = range.maxTsUnixMs - range.minTsUnixMs;
  if (span <= 0) return 0;
  return Math.min(1, Math.max(0, (tsUnixMs - range.minTsUnixMs) / span));
}

export interface YearTick {
  tsUnixMs: number;
  fraction: number;
  label: string;
}

/** One tick per calendar year boundary (UTC) that falls inside `range`. */
export function yearTicks(range: TimelineRange): YearTick[] {
  const startYear = new Date(range.minTsUnixMs).getUTCFullYear();
  const endYear = new Date(range.maxTsUnixMs).getUTCFullYear();

  const ticks: YearTick[] = [];
  for (let year = startYear; year <= endYear; year++) {
    const tsUnixMs = Date.UTC(year, 0, 1);
    if (tsUnixMs < range.minTsUnixMs || tsUnixMs > range.maxTsUnixMs) continue;
    ticks.push({ tsUnixMs, fraction: timestampToFraction(tsUnixMs, range), label: String(year) });
  }
  return ticks;
}

export interface MonthTick {
  tsUnixMs: number;
  fraction: number;
}

/** One tick per calendar month boundary (UTC) inside `range`, for fine ticks between year labels. */
export function monthTicks(range: TimelineRange): MonthTick[] {
  const ticks: MonthTick[] = [];
  const start = new Date(range.minTsUnixMs);
  let year = start.getUTCFullYear();
  let month = start.getUTCMonth();

  while (true) {
    const tsUnixMs = Date.UTC(year, month, 1);
    if (tsUnixMs > range.maxTsUnixMs) break;
    if (tsUnixMs >= range.minTsUnixMs) {
      ticks.push({ tsUnixMs, fraction: timestampToFraction(tsUnixMs, range) });
    }
    month += 1;
    if (month > 11) {
      month = 0;
      year += 1;
    }
  }
  return ticks;
}
