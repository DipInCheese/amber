// SPDX-License-Identifier: GPL-3.0-or-later
import { describe, expect, it } from "vitest";
import { addRecent, formatOpenedAt, MAX_RECENTS, removeRecent, type RecentArchive } from "./recents";

const entry = (path: string, openedAt = 1): RecentArchive => ({
  path,
  name: path,
  messageCount: 10,
  openedAt,
});

describe("recents", () => {
  it("puts the newest first and moves a reopened file to the top", () => {
    let list = addRecent([], entry("a"));
    list = addRecent(list, entry("b"));
    list = addRecent(list, entry("a", 5));
    expect(list.map((item) => item.path)).toEqual(["a", "b"]);
    expect(list[0].openedAt).toBe(5);
  });

  it("keeps only the most recent few", () => {
    let list: RecentArchive[] = [];
    for (let i = 0; i < MAX_RECENTS + 3; i += 1) list = addRecent(list, entry(`f${i}`));
    expect(list).toHaveLength(MAX_RECENTS);
    expect(list[0].path).toBe(`f${MAX_RECENTS + 2}`);
  });

  it("removes by path", () => {
    expect(removeRecent([entry("a"), entry("b")], "a").map((item) => item.path)).toEqual(["b"]);
  });

  it("describes how long ago in plain words", () => {
    const now = 1_700_000_000_000;
    expect(formatOpenedAt(now - 5_000, now)).toBe("Just now");
    expect(formatOpenedAt(now - 5 * 60_000, now)).toBe("5 minutes ago");
    expect(formatOpenedAt(now - 3 * 3_600_000, now)).toBe("3 hours ago");
    expect(formatOpenedAt(now - 24 * 3_600_000, now)).toBe("Yesterday");
    expect(formatOpenedAt(now - 3 * 24 * 3_600_000, now)).toBe("3 days ago");
  });
});
