// SPDX-License-Identifier: GPL-3.0-or-later
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import type { DayBucketDto, MessageDto, OpenArchiveResult } from "./types";

export function openArchive(path: string): Promise<OpenArchiveResult> {
  return invoke("open_archive", { path });
}

export function queryMessages(): Promise<MessageDto[]> {
  return invoke("query_messages");
}

export function getDayIndex(): Promise<DayBucketDto[]> {
  return invoke("get_day_index");
}

/**
 * URL for an attachment's bytes, served straight out of the open archive's
 * ZIP by the `amber-attachment` custom protocol (src-tauri/src/protocol.rs)
 * - never copied into the web layer. `convertFileSrc` builds the
 * platform-correct URL (the scheme prefix differs on Windows vs
 * macOS/Linux/Android) rather than hardcoding one form.
 */
export function attachmentUrl(relPath: string): string {
  return convertFileSrc(relPath, "amber-attachment");
}
