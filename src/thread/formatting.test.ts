// SPDX-License-Identifier: GPL-3.0-or-later
import { describe, expect, it } from "vitest";
import { formatDayHeading } from "./formatting";

describe("formatDayHeading", () => {
  const now = new Date(2025, 5, 18, 15, 0).getTime(); // Wed 18 June 2025, local

  it("says Today and Yesterday", () => {
    expect(formatDayHeading(new Date(2025, 5, 18, 0, 5).getTime(), now)).toBe("Today");
    expect(formatDayHeading(new Date(2025, 5, 17, 23, 59).getTime(), now)).toBe("Yesterday");
  });

  it("uses just the weekday within the last week", () => {
    const text = formatDayHeading(new Date(2025, 5, 14, 12).getTime(), now);
    expect(text).toBe(new Intl.DateTimeFormat(undefined, { weekday: "long" }).format(new Date(2025, 5, 14)));
  });

  it("adds the year only for other years", () => {
    const sameYear = formatDayHeading(new Date(2025, 0, 3, 12).getTime(), now);
    const otherYear = formatDayHeading(new Date(2023, 0, 3, 12).getTime(), now);
    expect(sameYear).not.toContain("2025");
    expect(otherYear).toContain("2023");
  });
});
