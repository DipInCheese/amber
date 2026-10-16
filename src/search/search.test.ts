// SPDX-License-Identifier: GPL-3.0-or-later
import { describe, expect, it } from "vitest";
import type { MessageDto } from "../lib/types";
import { findMatches, splitByQuery } from "./search";

function message(text: string | null, overrides: Partial<MessageDto> = {}): MessageDto {
  return {
    guid: null,
    ts_unix_ms: 0,
    sender_identifier: "+1",
    is_from_me: false,
    service: null,
    text,
    reply_to_guid: null,
    is_edited: false,
    is_unsent: false,
    edit_history: null,
    attachments: [],
    reactions: [],
    ...overrides,
  };
}

describe("findMatches", () => {
  const messages = [message("Dinner at the Café?"), message(null), message("see you there"), message("CAFE again")];

  it("ignores capitals and accents", () => {
    expect(findMatches(messages, "cafe")).toEqual([0, 3]);
  });

  it("matches nothing for an empty query", () => {
    expect(findMatches(messages, "  ")).toEqual([]);
  });

  it("skips unsent messages and messages without text", () => {
    expect(findMatches([message("hello", { is_unsent: true }), message(null)], "hello")).toEqual([]);
  });
});

describe("splitByQuery", () => {
  it("marks every occurrence and keeps the original text", () => {
    expect(splitByQuery("Tea or tea?", "tea")).toEqual([
      { text: "Tea", match: true },
      { text: " or ", match: false },
      { text: "tea", match: true },
      { text: "?", match: false },
    ]);
  });

  it("highlights across accents", () => {
    expect(splitByQuery("Un café noir", "cafe")).toEqual([
      { text: "Un ", match: false },
      { text: "café", match: true },
      { text: " noir", match: false },
    ]);
  });

  it("returns the text whole when nothing matches", () => {
    expect(splitByQuery("hello", "zzz")).toEqual([{ text: "hello", match: false }]);
  });
});
