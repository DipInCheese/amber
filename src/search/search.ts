// SPDX-License-Identifier: GPL-3.0-or-later
import type { MessageDto } from "../lib/types";

/** Lower-cases and strips accents, so "cafe" finds "Café" and "STRASSE" finds "straße"-less text alike. */
export function foldForSearch(text: string): string {
  return text.normalize("NFD").replace(/\p{M}+/gu, "").toLowerCase();
}

/**
 * Indices of the messages whose text contains `query`, in conversation
 * order. Accents and capitals are ignored. An empty query matches nothing.
 */
export function findMatches(messages: MessageDto[], query: string): number[] {
  const needle = foldForSearch(query.trim());
  if (!needle) return [];
  const matches: number[] = [];
  messages.forEach((message, index) => {
    if (message.is_unsent || !message.text) return;
    if (foldForSearch(message.text).includes(needle)) matches.push(index);
  });
  return matches;
}

export interface TextPart {
  text: string;
  match: boolean;
}

/**
 * Splits `text` so every occurrence of `query` can be highlighted. Works on
 * the folded text but cuts the original, which is safe because folding only
 * removes combining marks - done per character so offsets stay aligned.
 */
export function splitByQuery(text: string, query: string): TextPart[] {
  const needle = foldForSearch(query.trim());
  if (!needle) return [{ text, match: false }];

  const chars = Array.from(text);
  const folded = chars.map((char) => foldForSearch(char));
  const flat = folded.join("");
  const starts: number[] = [];
  let offset = 0;
  for (const piece of folded) {
    starts.push(offset);
    offset += piece.length;
  }

  const parts: TextPart[] = [];
  let cursor = 0; // index into chars
  let from = 0; // index into flat
  for (;;) {
    const at = flat.indexOf(needle, from);
    if (at === -1) break;
    const first = starts.findIndex((start, i) => start <= at && at < start + Math.max(folded[i].length, 1));
    let last = first;
    while (last + 1 < chars.length && starts[last + 1] < at + needle.length) last += 1;
    if (first === -1) break;
    if (first > cursor) parts.push({ text: chars.slice(cursor, first).join(""), match: false });
    parts.push({ text: chars.slice(first, last + 1).join(""), match: true });
    cursor = last + 1;
    from = at + needle.length;
  }
  if (cursor < chars.length) parts.push({ text: chars.slice(cursor).join(""), match: false });
  return parts.length > 0 ? parts : [{ text, match: false }];
}
