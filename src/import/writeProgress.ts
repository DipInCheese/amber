// SPDX-License-Identifier: GPL-3.0-or-later
import type { WriteProgress } from "../lib/api";

export interface WriteProgressView {
  label: string;
  /** 0-1 for a determinate bar, `null` when there's no meaningful fraction. */
  fraction: number | null;
}

/** What to show while the archive is being written. */
export function describeWriteProgress(progress: WriteProgress | null): WriteProgressView {
  if (progress === null) return { label: "Preparing the archive…", fraction: null };

  const counted = (verb: string) => ({
    label: `${verb} ${progress.done.toLocaleString()} of ${progress.total.toLocaleString()}`,
    fraction: progress.total > 0 ? progress.done / progress.total : null,
  });

  switch (progress.stage) {
    case "hashing":
      return counted("Reading attachments…");
    case "database":
      return { label: "Building the message database…", fraction: null };
    case "writing":
      return counted("Saving attachments…");
  }
}
