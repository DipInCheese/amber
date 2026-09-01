// SPDX-License-Identifier: GPL-3.0-or-later
import { describe, expect, it } from "vitest";
import {
  fractionToTimestamp,
  monthTicks,
  parseDayKey,
  rangeFromDayKeys,
  timestampToFraction,
  yearTicks,
} from "./timelineMath";

describe("parseDayKey", () => {
  it("parses a day key as UTC midnight", () => {
    expect(parseDayKey("2024-03-05")).toBe(Date.UTC(2024, 2, 5));
  });
});

describe("rangeFromDayKeys", () => {
  it("returns null for an empty conversation", () => {
    expect(rangeFromDayKeys([])).toBeNull();
  });

  it("spans from the earliest day to the end of the latest day", () => {
    const range = rangeFromDayKeys(["2024-03-05", "2020-01-01", "2024-06-01"]);
    expect(range).toEqual({
      minTsUnixMs: Date.UTC(2020, 0, 1),
      maxTsUnixMs: Date.UTC(2024, 5, 2), // end of 2024-06-01
    });
  });
});

describe("fractionToTimestamp / timestampToFraction", () => {
  const range = { minTsUnixMs: 0, maxTsUnixMs: 1000 };

  it("round-trips", () => {
    for (const fraction of [0, 0.25, 0.5, 0.75, 1]) {
      const ts = fractionToTimestamp(fraction, range);
      expect(timestampToFraction(ts, range)).toBeCloseTo(fraction);
    }
  });

  it("clamps out-of-range fractions", () => {
    expect(fractionToTimestamp(-1, range)).toBe(0);
    expect(fractionToTimestamp(2, range)).toBe(1000);
  });

  it("clamps out-of-range timestamps", () => {
    expect(timestampToFraction(-500, range)).toBe(0);
    expect(timestampToFraction(1500, range)).toBe(1);
  });

  it("does not divide by zero for a single-instant range", () => {
    expect(timestampToFraction(5, { minTsUnixMs: 5, maxTsUnixMs: 5 })).toBe(0);
  });
});

describe("yearTicks", () => {
  it("emits one tick per year boundary inside the range, spanning multiple years", () => {
    const range = rangeFromDayKeys(["2021-06-01", "2024-01-15"])!;
    const ticks = yearTicks(range);
    expect(ticks.map((t) => t.label)).toEqual(["2022", "2023", "2024"]);
    // Fractions must be strictly increasing (chronological order).
    for (let i = 1; i < ticks.length; i++) {
      expect(ticks[i].fraction).toBeGreaterThan(ticks[i - 1].fraction);
    }
  });

  it("emits nothing for a range that crosses no year boundary", () => {
    const range = rangeFromDayKeys(["2024-03-01", "2024-06-01"])!;
    expect(yearTicks(range)).toEqual([]);
  });
});

describe("monthTicks", () => {
  it("covers every month boundary from the first to the last", () => {
    const range = rangeFromDayKeys(["2024-01-01", "2024-04-01"])!;
    const ticks = monthTicks(range);
    expect(ticks.map((t) => new Date(t.tsUnixMs).getUTCMonth())).toEqual([0, 1, 2, 3]);
  });

  it("skips a month boundary that falls before the range starts", () => {
    // Range starts mid-January, so the Jan 1 boundary itself is out of range.
    const range = rangeFromDayKeys(["2024-01-15", "2024-04-01"])!;
    const ticks = monthTicks(range);
    expect(ticks.map((t) => new Date(t.tsUnixMs).getUTCMonth())).toEqual([1, 2, 3]);
  });
});
