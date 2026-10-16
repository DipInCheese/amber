// SPDX-License-Identifier: GPL-3.0-or-later

/** An archive the user opened before, remembered so the home screen can offer it again. */
export interface RecentArchive {
  path: string;
  name: string;
  messageCount: number;
  openedAt: number;
}

const KEY = "amber:recent-archives";
export const MAX_RECENTS = 8;

/** Puts `entry` first, replacing any earlier entry for the same file, keeping at most `max`. */
export function addRecent(list: RecentArchive[], entry: RecentArchive, max = MAX_RECENTS): RecentArchive[] {
  return [entry, ...list.filter((item) => item.path !== entry.path)].slice(0, max);
}

export function removeRecent(list: RecentArchive[], path: string): RecentArchive[] {
  return list.filter((item) => item.path !== path);
}

function isRecent(value: unknown): value is RecentArchive {
  if (typeof value !== "object" || value === null) return false;
  const v = value as Record<string, unknown>;
  return (
    typeof v.path === "string" &&
    typeof v.name === "string" &&
    typeof v.messageCount === "number" &&
    typeof v.openedAt === "number"
  );
}

export function loadRecents(): RecentArchive[] {
  try {
    const parsed: unknown = JSON.parse(localStorage.getItem(KEY) ?? "[]");
    return Array.isArray(parsed) ? parsed.filter(isRecent).slice(0, MAX_RECENTS) : [];
  } catch {
    return [];
  }
}

export function saveRecents(list: RecentArchive[]): void {
  try {
    localStorage.setItem(KEY, JSON.stringify(list));
  } catch {
    // Remembering recents is a convenience; losing them is fine.
  }
}

/** "Just now", "5 minutes ago", "Yesterday", or a short date - for the recents list. */
export function formatOpenedAt(openedAt: number, now: number = Date.now()): string {
  const seconds = Math.round((now - openedAt) / 1000);
  if (seconds < 45) return "Just now";
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return minutes === 1 ? "A minute ago" : `${minutes} minutes ago`;
  const hours = Math.round(minutes / 60);
  if (hours < 24) return hours === 1 ? "An hour ago" : `${hours} hours ago`;
  const days = Math.round(hours / 24);
  if (days === 1) return "Yesterday";
  if (days < 7) return `${days} days ago`;
  return new Date(openedAt).toLocaleDateString(undefined, { year: "numeric", month: "short", day: "numeric" });
}
