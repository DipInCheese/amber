// SPDX-License-Identifier: GPL-3.0-or-later
// SPEC.md stores ts_unix_ms as UTC; everything here renders in the
// viewer's local time zone, per SPEC.md's "Time representation" note.


const timeFormatter = new Intl.DateTimeFormat(undefined, {
  hour: "2-digit",
  minute: "2-digit",
});

const monthYearFormatter = new Intl.DateTimeFormat(undefined, {
  month: "long",
  year: "numeric",
  timeZone: "UTC",
});

/** Local calendar-day key (e.g. "2024-03-05"), for grouping into date separators. */
export function localDayKey(tsUnixMs: number): string {
  const d = new Date(tsUnixMs);
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(
    d.getDate(),
  ).padStart(2, "0")}`;
}


export function formatTime(tsUnixMs: number): string {
  return timeFormatter.format(new Date(tsUnixMs));
}

/** Formatted in UTC deliberately: the timeline scrubber's own date math (timelineMath.ts) works in UTC calendar days, matching how the backend buckets day_index - keeps the label consistent with where the thumb actually lands. */
export function formatMonthYear(tsUnixMs: number): string {
  return monthYearFormatter.format(new Date(tsUnixMs));
}

const weekdayFormatter = new Intl.DateTimeFormat(undefined, { weekday: "long" });
const dayMonthFormatter = new Intl.DateTimeFormat(undefined, { weekday: "long", month: "long", day: "numeric" });
const fullDayFormatter = new Intl.DateTimeFormat(undefined, {
  weekday: "long",
  month: "long",
  day: "numeric",
  year: "numeric",
});

function startOfLocalDay(ms: number): number {
  const d = new Date(ms);
  return new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime();
}

/**
 * The heading above a day's messages, the way Messages words it: "Today",
 * "Yesterday", the weekday for the last week, then the date (with the year
 * once it isn't this year).
 */
export function formatDayHeading(tsUnixMs: number, now: number = Date.now()): string {
  const days = Math.round((startOfLocalDay(now) - startOfLocalDay(tsUnixMs)) / 86_400_000);
  if (days === 0) return "Today";
  if (days === 1) return "Yesterday";
  const date = new Date(tsUnixMs);
  if (days > 1 && days < 7) return weekdayFormatter.format(date);
  return date.getFullYear() === new Date(now).getFullYear()
    ? dayMonthFormatter.format(date)
    : fullDayFormatter.format(date);
}
