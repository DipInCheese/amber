// SPDX-License-Identifier: GPL-3.0-or-later
// Mirrors the DTOs in src-tauri/src/dto.rs.

export interface ParticipantDto {
  identifier: string;
  display_name: string | null;
  is_me: boolean;
}

export interface AttachmentDto {
  rel_path: string;
  mime_type: string | null;
  filename: string | null;
  width: number | null;
  height: number | null;
  duration_ms: number | null;
}

export interface ReactionDto {
  participant_identifier: string | null;
  kind: string;
  emoji: string | null;
  ts_unix_ms: number | null;
  is_removed: boolean;
}

export interface MessageDto {
  guid: string | null;
  ts_unix_ms: number;
  sender_identifier: string | null;
  is_from_me: boolean;
  text: string | null;
  reply_to_guid: string | null;
  is_edited: boolean;
  is_unsent: boolean;
  edit_history: string[] | null;
  attachments: AttachmentDto[];
  reactions: ReactionDto[];
}

export interface DayBucketDto {
  day: string;
  message_count: number;
}

export interface OpenArchiveResult {
  chat_identifier: string;
  display_name: string | null;
  is_group: boolean;
  participants: ParticipantDto[];
  message_count: number;
}

export interface PrerequisiteStatusDto {
  ok: boolean;
  guidance: string | null;
}

export interface PrerequisiteReportDto {
  idevice_id: PrerequisiteStatusDto;
  idevicebackup2: PrerequisiteStatusDto;
  platform: PrerequisiteStatusDto;
}

export interface ConversationSummaryDto {
  chat_identifier: string;
  display_name: string | null;
  is_group: boolean;
  message_count: number;
}

export type SourceRequest =
  | { type: "device"; udid: string | null; password: string | null }
  | { type: "backup"; path: string; password: string | null }
  | { type: "live_mac" };
