// SPDX-License-Identifier: GPL-3.0-or-later
// SPEC.md stores ts_unix_ms as UTC; everything here renders in the
// viewer's local time zone, per SPEC.md's "Time representation" note.

const dateFormatter = new Intl.DateTimeFormat(undefined, {
  weekday: "long",
  month: "long",
  day: "numeric",
  year: "numeric",
});

const timeFormatter = new Intl.DateTimeFormat(undefined, {
  hour: "numeric",
  minute: "2-digit",
});

/** Local calendar-day key (e.g. "2024-03-05"), for grouping into date separators. */
export function localDayKey(tsUnixMs: number): string {
  const d = new Date(tsUnixMs);
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(
    d.getDate(),
  ).padStart(2, "0")}`;
}

export function formatDateSeparator(tsUnixMs: number): string {
  return dateFormatter.format(new Date(tsUnixMs));
}

export function formatTime(tsUnixMs: number): string {
  return timeFormatter.format(new Date(tsUnixMs));
}
