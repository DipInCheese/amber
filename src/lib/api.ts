// SPDX-License-Identifier: GPL-3.0-or-later
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  ConversationSummaryDto,
  DayBucketDto,
  DeviceBackupCacheDto,
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

/** The newest complete phone backup Amber is holding between imports, if any. */
export function deviceBackupCacheInfo(): Promise<DeviceBackupCacheDto | null> {
  return invoke("device_backup_cache_info");
}

export function clearDeviceBackupCache(): Promise<void> {
  return invoke("clear_device_backup_cache");
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

export interface WriteProgress {
  stage: "hashing" | "database" | "writing";
  done: number;
  total: number;
}

/** Progress while `ingest_conversation` is writing the `.amber` archive. */
export function onWriteProgress(handler: (progress: WriteProgress) => void): Promise<UnlistenFn> {
  return listen<WriteProgress>("ingest://write-progress", (event) => handler(event.payload));
}

/** 0-100 progress while `begin_ingest` is creating/refreshing a device backup. */
export function onBackupProgress(handler: (percent: number) => void): Promise<UnlistenFn> {
  return listen<number>("ingest://backup-progress", (event) => handler(event.payload));
}

/** Write one attachment, exactly as stored, to a path the user picked. */
export function saveAttachment(relPath: string, destPath: string): Promise<void> {
  return invoke("save_attachment", { relPath, destPath });
}

export type ExportFormat = "text" | "csv" | "json" | "html";

export interface ExportSummary {
  path: string;
  messages: number;
  media_files: number;
  media_failed: number;
}

export function exportConversation(
  format: ExportFormat,
  includeMedia: boolean,
  outputPath: string,
): Promise<ExportSummary> {
  return invoke("export_conversation", { format, includeMedia, outputPath });
}

export interface ExportProgress {
  done: number;
  total: number;
}

export function onExportProgress(handler: (progress: ExportProgress) => void): Promise<UnlistenFn> {
  return listen<ExportProgress>("export://progress", (event) => handler(event.payload));
}
