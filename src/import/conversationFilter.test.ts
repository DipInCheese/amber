// SPDX-License-Identifier: GPL-3.0-or-later
import { describe, expect, it } from "vitest";
import type { ConversationSummaryDto } from "../lib/types";
import { filterConversations, formatBytes, formatLastActive } from "./conversationFilter";

function conversation(
  chat_identifier: string,
  display_name: string | null = null,
): ConversationSummaryDto {
  return {
    chat_identifier,
    display_name,
    is_group: false,
    message_count: 1,
    last_message_ts_unix_ms: null,
  };
}

const list = [
  conversation("+15551234567"),
  conversation("chat123456", "Family"),
  conversation("someone@example.com"),
];

describe("filterConversations", () => {
  it("returns everything for an empty or blank query", () => {
    expect(filterConversations(list, "")).toEqual(list);
    expect(filterConversations(list, "   ")).toEqual(list);
  });

  it("matches display names case-insensitively", () => {
    expect(filterConversations(list, "fAmI").map((c) => c.chat_identifier)).toEqual(["chat123456"]);
  });

  it("matches identifiers such as email addresses", () => {
    expect(filterConversations(list, "example.com").map((c) => c.chat_identifier)).toEqual([
      "someone@example.com",
    ]);
  });

  it("matches phone numbers regardless of formatting", () => {
    expect(filterConversations(list, "(555) 123-4567").map((c) => c.chat_identifier)).toEqual([
      "+15551234567",
    ]);
  });

  it("returns nothing when no conversation matches", () => {
    expect(filterConversations(list, "zzz")).toEqual([]);
  });
});

describe("formatBytes", () => {
  it("scales to a readable unit", () => {
    expect(formatBytes(512)).toBe("512 B");
    expect(formatBytes(1536)).toBe("1.5 KB");
    expect(formatBytes(5 * 1024 * 1024 * 1024)).toBe("5.0 GB");
  });
});

describe("formatLastActive", () => {
  it("says so when there are no messages", () => {
    expect(formatLastActive(null)).toBe("No messages");
  });

  it("formats a timestamp as a date", () => {
    expect(formatLastActive(Date.UTC(2024, 5, 15, 12))).toMatch(/2024/);
  });
});
