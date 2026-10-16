// SPDX-License-Identifier: GPL-3.0-or-later
import { describe, expect, it } from "vitest";
import { easeToward, SCROLL_SNAP_PX, SCROLL_TAU_MS } from "./smoothScroll";

describe("easeToward", () => {
  it("moves toward the target without overshooting, in either direction", () => {
    const down = easeToward(0, 1000, 16);
    expect(down).toBeGreaterThan(0);
    expect(down).toBeLessThan(1000);

    const up = easeToward(1000, 0, 16);
    expect(up).toBeLessThan(1000);
    expect(up).toBeGreaterThan(0);
  });

  it("covers about 63% of the distance in one time constant", () => {
    const after = easeToward(0, 1000, SCROLL_TAU_MS);
    expect(after).toBeCloseTo(632, -1);
  });

  it("looks the same regardless of frame rate", () => {
    const oneBigStep = easeToward(0, 1000, 32);
    const twoSmallSteps = easeToward(easeToward(0, 1000, 16), 1000, 16);
    expect(twoSmallSteps).toBeCloseTo(oneBigStep, 6);
  });

  it("arrives exactly and stays put", () => {
    let position = 0;
    for (let frame = 0; frame < 400; frame++) position = easeToward(position, 5000, 16);
    expect(position).toBe(5000);
    expect(easeToward(5000, 5000, 16)).toBe(5000);
  });

  it("snaps when already within a pixel instead of crawling forever", () => {
    expect(easeToward(100, 100 + SCROLL_SNAP_PX / 2, 16)).toBe(100 + SCROLL_SNAP_PX / 2);
  });

  it("doesn't move on a zero-length frame", () => {
    expect(easeToward(0, 1000, 0)).toBe(0);
  });

  it("is monotonic: every frame is at least as close as the last", () => {
    let position = 0;
    let last = Infinity;
    for (let frame = 0; frame < 100; frame++) {
      position = easeToward(position, 12345, 16);
      const distance = Math.abs(12345 - position);
      expect(distance).toBeLessThanOrEqual(last);
      last = distance;
    }
  });
});
