// SPDX-License-Identifier: GPL-3.0-or-later
import { describe, expect, it } from "vitest";
import { describeExport, describeExportProgress, EXPORT_FORMATS } from "./exportSummary";

const base = { path: "C:/out.html", messages: 1, media_files: 0, media_failed: 0 };

describe("describeExport", () => {
  it("reports a text-only export", () => {
    expect(describeExport(base)).toBe("Exported 1 message.");
  });

  it("mentions copied media", () => {
    const text = describeExport({ ...base, messages: 5, media_files: 3 });
    expect(text).toContain("Exported 5 messages.");
    expect(text).toContain("3 photo, video or other files");
  });

  it("never hides files that couldn't be copied", () => {
    const text = describeExport({ ...base, media_files: 10, media_failed: 2 });
    expect(text).toContain("2 files couldn't be copied");
  });
});

describe("EXPORT_FORMATS", () => {
  it("offers each format once with a distinct extension", () => {
    expect(new Set(EXPORT_FORMATS.map((f) => f.id)).size).toBe(EXPORT_FORMATS.length);
    expect(new Set(EXPORT_FORMATS.map((f) => f.extension)).size).toBe(EXPORT_FORMATS.length);
  });
});

describe("describeExportProgress", () => {
  it("counts the steps, with a media step only when media is copied", () => {
    expect(describeExportProgress(null, true)).toMatchObject({ step: "Step 1 of 3", fraction: null });
    expect(describeExportProgress({ stage: "media", done: 5, total: 20 }, true)).toMatchObject({
      step: "Step 2 of 3",
      fraction: 0.25,
    });
    const writing = describeExportProgress({ stage: "writing", done: 50, total: 100 }, false);
    expect(writing).toMatchObject({ step: "Step 2 of 2", fraction: 0.5 });
    expect(writing.label).toContain("Writing messages");
  });

  it("has no fraction for an empty stage", () => {
    expect(describeExportProgress({ stage: "writing", done: 0, total: 0 }, false).fraction).toBeNull();
  });
});
