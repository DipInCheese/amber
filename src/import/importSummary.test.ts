// SPDX-License-Identifier: GPL-3.0-or-later
import { describe, expect, it } from "vitest";
import type { OpenArchiveResult } from "../lib/types";
import { importSummary } from "./importSummary";

function result(
  attachments_imported: number | null,
  attachments_missing: number | null,
): OpenArchiveResult {
  return {
    chat_identifier: "+15551234567",
    display_name: null,
    is_group: false,
    participants: [],
    message_count: 1,
    attachments_imported,
    attachments_missing,
  };
}

describe("importSummary", () => {
  it("says nothing for an archive that wasn't just imported", () => {
    expect(importSummary(result(null, null))).toBeNull();
  });

  it("says nothing for a text-only conversation", () => {
    expect(importSummary(result(0, 0))).toBeNull();
  });

  it("reports a clean import", () => {
    expect(importSummary(result(1, 0))).toBe("Imported 1 attachment.");
    // Counts follow the user's locale ("4,908" / "4.908"), so build the
    // expectation the same way instead of assuming one.
    expect(importSummary(result(4908, 0))).toBe(
      `Imported ${(4908).toLocaleString()} attachments.`,
    );
  });

  it("calls out attachments the source didn't have", () => {
    const text = importSummary(result(4908, 970));
    expect(text).toContain(`Imported ${(4908).toLocaleString()} attachments.`);
    expect(text).toContain("970 other attachments");
    expect(text).toContain("iCloud");
  });

  it("still reports when nothing at all could be found", () => {
    expect(importSummary(result(0, 12))).toContain("12 other attachments");
  });
});
