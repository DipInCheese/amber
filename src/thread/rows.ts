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
