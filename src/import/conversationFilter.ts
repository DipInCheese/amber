// SPDX-License-Identifier: GPL-3.0-or-later
import type { ConversationSummaryDto } from "../lib/types";

/**
 * Narrows the conversation list to what the user typed. Matches the display
 * name or identifier as plain text, and also compares digits only, so
 * "555 123" finds "+15551234567" regardless of how either side is formatted.
 */
export function filterConversations(
  conversations: ConversationSummaryDto[],
  query: string,
): ConversationSummaryDto[] {
  const text = query.trim().toLowerCase();
  if (!text) return conversations;

  const digits = text.replace(/\D/g, "");
  return conversations.filter((conversation) => {
    const name = (conversation.display_name ?? "").toLowerCase();
    const identifier = conversation.chat_identifier.toLowerCase();
    if (name.includes(text) || identifier.includes(text)) return true;
    return digits.length > 0 && conversation.chat_identifier.replace(/\D/g, "").includes(digits);
  });
}

export function formatBytes(bytes: number): string {
  const units = ["B", "KB", "MB", "GB", "TB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return `${unit === 0 ? value : value.toFixed(1)} ${units[unit]}`;
}

export function formatLastActive(timestampMs: number | null): string {
  if (timestampMs === null) return "No messages";
  return new Date(timestampMs).toLocaleDateString(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
  });
}
