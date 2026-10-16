// SPDX-License-Identifier: GPL-3.0-or-later
import { describe, expect, it } from "vitest";
import { describeWriteProgress } from "./writeProgress";

describe("describeWriteProgress", () => {
  it("shows a neutral message before the first update", () => {
    expect(describeWriteProgress(null)).toEqual({ label: "Preparing the archive…", fraction: null });
  });

  it("reports attachment counts and a fraction while hashing and saving", () => {
    const hashing = describeWriteProgress({ stage: "hashing", done: 25, total: 100 });
    expect(hashing.fraction).toBe(0.25);
    expect(hashing.label).toContain("Reading attachments");

    const writing = describeWriteProgress({ stage: "writing", done: 3, total: 4 });
    expect(writing.fraction).toBe(0.75);
    expect(writing.label).toContain("Saving attachments");
  });

  it("has no fraction for the database stage", () => {
    expect(describeWriteProgress({ stage: "database", done: 0, total: 0 }).fraction).toBeNull();
  });

  it("doesn't divide by zero when there are no attachments", () => {
    expect(describeWriteProgress({ stage: "hashing", done: 0, total: 0 }).fraction).toBeNull();
  });
});
