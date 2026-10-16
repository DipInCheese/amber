// SPDX-License-Identifier: GPL-3.0-or-later
import type { MessageDto } from "../lib/types";
import { localDayKey } from "./formatting";

/** Where a bubble sits in a run of messages from the same person. */
export type RunPosition = "single" | "first" | "middle" | "last";

export type ThreadRow =
  | { kind: "date"; key: string; tsUnixMs: number }
  | {
      kind: "message";
      key: string;
      message: MessageDto;
      replyTo: MessageDto | null;
      position: RunPosition;
      /** A time stamp to show above this bubble: the pause since the last message was long. */
      timeAbove: number | null;
    };

/** Messages this close together from one person read as one burst: tight spacing, one tail. */
export const RUN_GAP_MS = 5 * 60_000;
/** A pause this long gets its own time stamp, like Messages. */
export const TIME_GAP_MS = 15 * 60_000;

function sameSender(a: MessageDto, b: MessageDto): boolean {
  if (a.is_from_me !== b.is_from_me) return false;
  return a.is_from_me || a.sender_identifier === b.sender_identifier;
}

/**
 * Flattens messages into the list of rows the virtualizer renders: a day
 * header at the start of each calendar day, then the messages. Consecutive
 * messages from the same person close in time form a run (tight spacing,
 * tail only on the last bubble); a long pause adds a time stamp.
 */
export function buildThreadRows(messages: MessageDto[]): ThreadRow[] {
  const byGuid = new Map<string, MessageDto>();
  for (const message of messages) {
    if (message.guid) byGuid.set(message.guid, message);
  }

  const rows: ThreadRow[] = [];
  let lastDayKey: string | null = null;

  messages.forEach((message, index) => {
    const previous = index > 0 ? messages[index - 1] : null;
    const next = index + 1 < messages.length ? messages[index + 1] : null;

    const dayKey = localDayKey(message.ts_unix_ms);
    const startsDay = dayKey !== lastDayKey;
    if (startsDay) {
      rows.push({ kind: "date", key: `date-${dayKey}`, tsUnixMs: message.ts_unix_ms });
      lastDayKey = dayKey;
    }

    const joinsPrevious =
      !startsDay &&
      previous !== null &&
      sameSender(previous, message) &&
      message.ts_unix_ms - previous.ts_unix_ms < RUN_GAP_MS;
    const joinsNext =
      next !== null &&
      localDayKey(next.ts_unix_ms) === dayKey &&
      sameSender(message, next) &&
      next.ts_unix_ms - message.ts_unix_ms < RUN_GAP_MS;

    const position: RunPosition = joinsPrevious ? (joinsNext ? "middle" : "last") : joinsNext ? "first" : "single";
    const longPause =
      !startsDay && previous !== null && message.ts_unix_ms - previous.ts_unix_ms >= TIME_GAP_MS;

    rows.push({
      kind: "message",
      key: messageRowKey(message, index),
      message,
      replyTo: message.reply_to_guid ? (byGuid.get(message.reply_to_guid) ?? null) : null,
      position,
      timeAbove: longPause ? message.ts_unix_ms : null,
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

/** The key a message's row has in the thread (and that find uses to scroll to it). */
export function messageRowKey(message: MessageDto, index: number): string {
  return message.guid ?? `msg-${index}`;
}
