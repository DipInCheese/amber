// SPDX-License-Identifier: GPL-3.0-or-later
import { describe, expect, it } from "vitest";
import type { MessageDto } from "../lib/types";
import { buildThreadRows, findRowIndexForTimestamp, type ThreadRow } from "./rows";

function message(overrides: Partial<MessageDto> & { guid: string; ts_unix_ms: number }): MessageDto {
  return {
    sender_identifier: "+15551234567",
    is_from_me: false,
    text: "hi",
    reply_to_guid: null,
    is_edited: false,
    is_unsent: false,
    edit_history: null,
    service: "iMessage",
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

const minutes = (n: number) => n * 60_000;
const positions = (rows: ThreadRow[]) => rows.flatMap((r) => (r.kind === "message" ? [r.position] : []));

describe("message runs", () => {
  const base = Date.UTC(2024, 5, 3, 12);

  it("groups a burst from one person and tails only the last bubble", () => {
    const rows = buildThreadRows([
      message({ guid: "a", ts_unix_ms: base }),
      message({ guid: "b", ts_unix_ms: base + minutes(1) }),
      message({ guid: "c", ts_unix_ms: base + minutes(2) }),
    ]);
    expect(positions(rows)).toEqual(["first", "middle", "last"]);
  });

  it("starts a new run when the sender changes or the pause passes five minutes", () => {
    const rows = buildThreadRows([
      message({ guid: "a", ts_unix_ms: base }),
      message({ guid: "b", ts_unix_ms: base + minutes(1), is_from_me: true }),
      message({ guid: "c", ts_unix_ms: base + minutes(2), is_from_me: true }),
      message({ guid: "d", ts_unix_ms: base + minutes(9), is_from_me: true }),
    ]);
    expect(positions(rows)).toEqual(["single", "first", "last", "single"]);
  });

  it("tells different senders in a group chat apart", () => {
    const rows = buildThreadRows([
      message({ guid: "a", ts_unix_ms: base, sender_identifier: "+1" }),
      message({ guid: "b", ts_unix_ms: base + minutes(1), sender_identifier: "+2" }),
    ]);
    expect(positions(rows)).toEqual(["single", "single"]);
  });

  it("shows a time only after a pause of fifteen minutes or more", () => {
    const rows = buildThreadRows([
      message({ guid: "a", ts_unix_ms: base }),
      message({ guid: "b", ts_unix_ms: base + minutes(14) }),
      message({ guid: "c", ts_unix_ms: base + minutes(14) + minutes(15) }),
    ]);
    const times = rows.flatMap((r) => (r.kind === "message" ? [r.timeAbove] : []));
    expect(times).toEqual([null, null, base + minutes(29)]);
  });

  it("never joins a run across midnight", () => {
    const rows = buildThreadRows([
      message({ guid: "a", ts_unix_ms: new Date(2024, 5, 3, 23, 58).getTime() }),
      message({ guid: "b", ts_unix_ms: new Date(2024, 5, 4, 0, 1).getTime() }),
    ]);
    expect(positions(rows)).toEqual(["single", "single"]);
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
