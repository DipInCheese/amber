// SPDX-License-Identifier: GPL-3.0-or-later
import type { OpenArchiveResult } from "../lib/types";

function plural(count: number, noun: string): string {
  return `${count.toLocaleString()} ${noun}${count === 1 ? "" : "s"}`;
}

/**
 * One-line outcome of an import for the user, or `null` when there's
 * nothing to say (an archive opened normally, or a text-only conversation).
 * The "missing" half matters: attachments the source doesn't contain are
 * left out of the archive, and that must never happen silently.
 */
export function importSummary(result: OpenArchiveResult): string | null {
  const imported = result.attachments_imported;
  const missing = result.attachments_missing;
  if (imported === null || missing === null) return null;
  if (imported === 0 && missing === 0) return null;

  const done = `Imported ${plural(imported, "attachment")}.`;
  if (missing === 0) return done;

  return (
    `${done} ${plural(missing, "other attachment")} ` +
    "weren't in the backup - usually photos and videos that are stored only in iCloud - " +
    "so they're missing from this archive."
  );
}
