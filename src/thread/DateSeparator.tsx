// SPDX-License-Identifier: GPL-3.0-or-later
import { formatDayHeading, formatTime } from "./formatting";

/** "Today 14:32": the day in bold, the time of its first message beside it. */
export function DateSeparator({ tsUnixMs }: { tsUnixMs: number }) {
  return (
    <div className="date-separator">
      <strong>{formatDayHeading(tsUnixMs)}</strong> {formatTime(tsUnixMs)}
    </div>
  );
}
