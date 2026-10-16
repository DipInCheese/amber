// SPDX-License-Identifier: GPL-3.0-or-later
import type { ExportFormat, ExportSummary } from "../lib/api";

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
