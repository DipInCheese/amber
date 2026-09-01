// SPDX-License-Identifier: GPL-3.0-or-later
import { describe, expect, it } from "vitest";
import type { MessageDto } from "../lib/types";
import { buildThreadRows, findRowIndexForTimestamp } from "./rows";

function message(overrides: Partial<MessageDto> & { guid: string; ts_unix_ms: number }): MessageDto {
  return {
    sender_identifier: "+15551234567",
    is_from_me: false,
    text: "hi",
    reply_to_guid: null,
    is_edited: false,
    is_unsent: false,
    edit_history: null,
    attachments: [],
    reactions: [],
    ...overrides,
  };
}

describe("buildThreadRows", () => {
  it("inserts one date separator per calendar day and resolves replies by guid", () => {
    const messages: MessageDto[] = [
      message({ guid: "a", ts_unix_ms: Date.UTC(2024, 0, 1, 10) }),
      message({ guid: "b", ts_unix_ms: Date.UTC(2024, 0, 1, 11), reply_to_guid: "a" }),
      message({ guid: "c", ts_unix_ms: Date.UTC(2024, 0, 2, 9) }),
    ];

    const rows = buildThreadRows(messages);

    expect(rows.map((r) => r.kind)).toEqual(["date", "message", "message", "date", "message"]);

    const replyRow = rows.find((r) => r.kind === "message" && r.message.guid === "b");
    expect(replyRow?.kind === "message" && replyRow.replyTo?.guid).toBe("a");
  });
});

describe("findRowIndexForTimestamp", () => {
  it("finds the first row at or after the target timestamp", () => {
    const messages: MessageDto[] = [
      message({ guid: "a", ts_unix_ms: 1000 }),
      message({ guid: "b", ts_unix_ms: 2000 }),
      message({ guid: "c", ts_unix_ms: 3000 }),
    ];
    const rows = buildThreadRows(messages);

    // Just after "b" should land on "b"'s row (the row at or after 2500 is c, since 2500 > 2000).
    const indexAtB = findRowIndexForTimestamp(rows, 2000);
    expect(rows[indexAtB].kind === "message" && rows[indexAtB].message.guid).toBe("b");
  });

  it("returns the last index when every row is before the target", () => {
    const messages: MessageDto[] = [message({ guid: "a", ts_unix_ms: 1000 })];
    const rows = buildThreadRows(messages);
    expect(findRowIndexForTimestamp(rows, 999_999)).toBe(rows.length - 1);
  });

  it("returns 0 for an empty thread", () => {
    expect(findRowIndexForTimestamp([], 123)).toBe(0);
  });
});
