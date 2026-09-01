// SPDX-License-Identifier: GPL-3.0-or-later
import type { ReactionDto } from "../lib/types";

/** Maps a `reaction.kind` (SPEC.md: love|like|dislike|laugh|emphasize|question|emoji, tolerant of past-tense forms) to a display glyph. */
export function reactionGlyph(reaction: ReactionDto): string {
  const kind = reaction.kind.toLowerCase();
  if (kind.startsWith("love")) return "❤️";
  if (kind.startsWith("like")) return "👍";
  if (kind.startsWith("dislike")) return "👎";
  if (kind.startsWith("laugh")) return "😂";
  if (kind.startsWith("emphasi")) return "‼️";
  if (kind.startsWith("question")) return "❓";
  if (kind.startsWith("emoji")) return reaction.emoji ?? "⭐";
  if (kind.startsWith("sticker")) return "🏷️";
  return reaction.emoji ?? kind;
}
