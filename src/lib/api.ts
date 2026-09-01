// SPDX-License-Identifier: GPL-3.0-or-later
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  ConversationSummaryDto,
  DayBucketDto,
  MessageDto,
  OpenArchiveResult,
  PrerequisiteReportDto,
  SourceRequest,
} from "./types";

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

// M5: plug-and-play ingest

export function checkIngestPrerequisites(): Promise<PrerequisiteReportDto> {
  return invoke("check_ingest_prerequisites");
}

export function listIngestDevices(): Promise<string[]> {
  return invoke("list_ingest_devices");
}

export function beginIngest(source: SourceRequest): Promise<ConversationSummaryDto[]> {
  return invoke("begin_ingest", { source });
}

export function ingestConversation(
  chatIdentifier: string,
  outputPath: string,
): Promise<OpenArchiveResult> {
  return invoke("ingest_conversation", { chatIdentifier, outputPath });
}

/** 0-100 progress while `begin_ingest` is creating/refreshing a device backup. */
export function onBackupProgress(handler: (percent: number) => void): Promise<UnlistenFn> {
  return listen<number>("ingest://backup-progress", (event) => handler(event.payload));
}
