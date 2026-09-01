// SPDX-License-Identifier: GPL-3.0-or-later
import type { MessageDto } from "../lib/types";
import { localDayKey } from "./formatting";

export type ThreadRow =
  | { kind: "date"; key: string; tsUnixMs: number }
  | { kind: "message"; key: string; message: MessageDto; replyTo: MessageDto | null };

/**
 * Flattens messages into a single list of date-separator and message rows,
 * ready for the virtualizer - it doesn't know about "groups", just a flat
 * sequence of rows to measure and render.
 */
export function buildThreadRows(messages: MessageDto[]): ThreadRow[] {
  const byGuid = new Map<string, MessageDto>();
  for (const message of messages) {
    if (message.guid) byGuid.set(message.guid, message);
  }

  const rows: ThreadRow[] = [];
  let lastDayKey: string | null = null;

  messages.forEach((message, index) => {
    const dayKey = localDayKey(message.ts_unix_ms);
    if (dayKey !== lastDayKey) {
      rows.push({ kind: "date", key: `date-${dayKey}`, tsUnixMs: message.ts_unix_ms });
      lastDayKey = dayKey;
    }

    rows.push({
      kind: "message",
      key: message.guid ?? `msg-${index}`,
      message,
      replyTo: message.reply_to_guid ? (byGuid.get(message.reply_to_guid) ?? null) : null,
    });
  });

  return rows;
}

export function rowTimestamp(row: ThreadRow): number {
  return row.kind === "date" ? row.tsUnixMs : row.message.ts_unix_ms;
}

/**
 * Index of the first row at or after `targetTsUnixMs` (binary search - rows
 * are chronological). Used by the timeline scrubber to jump the virtualized
 * list to a date. Returns the last index if every row is before the target.
 */
export function findRowIndexForTimestamp(rows: ThreadRow[], targetTsUnixMs: number): number {
  if (rows.length === 0) return 0;

  let lo = 0;
  let hi = rows.length - 1;
  while (lo < hi) {
    const mid = (lo + hi) >> 1;
    if (rowTimestamp(rows[mid]) < targetTsUnixMs) {
      lo = mid + 1;
    } else {
      hi = mid;
    }
  }
  return lo;
}
