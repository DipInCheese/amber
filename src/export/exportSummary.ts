// SPDX-License-Identifier: GPL-3.0-or-later
import type { ExportFormat, ExportProgress, ExportSummary } from "../lib/api";

export interface ExportFormatInfo {
  id: ExportFormat;
  label: string;
  extension: string;
  description: string;
}

export const EXPORT_FORMATS: ExportFormatInfo[] = [
  {
    id: "html",
    label: "Web page (.html)",
    extension: "html",
    description: "Looks like the conversation. Opens in any browser; can include photos and videos.",
  },
  {
    id: "text",
    label: "Plain text (.txt)",
    extension: "txt",
    description: "Simple and readable anywhere. Photos are listed by name.",
  },
  {
    id: "csv",
    label: "Spreadsheet (.csv)",
    extension: "csv",
    description: "One row per message, for Excel or Google Sheets.",
  },
  {
    id: "json",
    label: "Data (.json)",
    extension: "json",
    description: "Everything in a structured form, for developers and other tools.",
  },
];

function plural(count: number, noun: string): string {
  return `${count.toLocaleString()} ${noun}${count === 1 ? "" : "s"}`;
}

/** One line telling the user what an export produced - including media that couldn't be copied. */
export function describeExport(summary: ExportSummary): string {
  let text = `Exported ${plural(summary.messages, "message")}.`;
  if (summary.media_files > 0) {
    text += ` Copied ${plural(summary.media_files, "photo, video or other file")} into a folder next to it.`;
  }
  if (summary.media_failed > 0) {
    text += ` ${plural(summary.media_failed, "file")} couldn't be copied and ${
      summary.media_failed === 1 ? "is" : "are"
    } only named in the page.`;
  }
  return text;
}

export interface ExportProgressView {
  /** What is happening, with counts. */
  label: string;
  /** "Step 2 of 3". */
  step: string;
  /** 0-1 for a determinate bar, `null` when there's no meaningful fraction. */
  fraction: number | null;
}

/**
 * What to show while an export runs. `withMedia` says whether a
 * media-copying step comes before the messages are written.
 */
export function describeExportProgress(progress: ExportProgress | null, withMedia: boolean): ExportProgressView {
  const steps = withMedia ? 3 : 2;
  if (progress === null || progress.stage === "reading") {
    return { label: "Reading the conversation…", step: `Step 1 of ${steps}`, fraction: null };
  }
  const counted = (verb: string, step: number): ExportProgressView => ({
    label: `${verb} ${progress.done.toLocaleString()} of ${progress.total.toLocaleString()}`,
    step: `Step ${step} of ${steps}`,
    fraction: progress.total > 0 ? progress.done / progress.total : null,
  });
  return progress.stage === "media" ? counted("Copying media…", 2) : counted("Writing messages…", steps);
}
