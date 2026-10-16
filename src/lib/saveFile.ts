// SPDX-License-Identifier: GPL-3.0-or-later
import { save } from "@tauri-apps/plugin-dialog";
import { saveAttachment } from "./api";

export type SaveResult = { status: "saved"; path: string } | { status: "cancelled" };

/** Ask where to save, then write the attachment there exactly as stored (a HEIC stays HEIC). */
export async function saveAttachmentWithDialog(relPath: string, filename: string): Promise<SaveResult> {
  const dot = filename.lastIndexOf(".");
  const extension = dot > 0 ? filename.slice(dot + 1) : null;

  const path = await save({
    defaultPath: filename,
    filters: extension ? [{ name: extension.toUpperCase(), extensions: [extension] }] : undefined,
  });
  if (!path) return { status: "cancelled" };

  await saveAttachment(relPath, path);
  return { status: "saved", path };
}
