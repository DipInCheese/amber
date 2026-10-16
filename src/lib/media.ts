// SPDX-License-Identifier: GPL-3.0-or-later
import type { AttachmentDto, MessageDto } from "./types";

export type MediaKind = "image" | "video" | "audio" | "file";

export function mediaKind(mimeType: string | null): MediaKind {
  const mime = mimeType ?? "";
  if (mime.startsWith("image/")) return "image";
  if (mime.startsWith("video/")) return "video";
  if (mime.startsWith("audio/")) return "audio";
  return "file";
}

/** One photo or video in the conversation, as the full-screen viewer steps through them. */
export interface MediaItem {
  /** Unique per occurrence: the same file can be attached to several messages. */
  key: string;
  relPath: string;
  kind: "image" | "video";
  filename: string;
  tsUnixMs: number;
  /** Who sent it, or `null` for the archive's owner. */
  sender: string | null;
  isFromMe: boolean;
}

/** The name to offer when saving: the real filename, else the archive's stored one. */
export function attachmentFilename(attachment: Pick<AttachmentDto, "filename" | "rel_path">): string {
  if (attachment.filename) return attachment.filename;
  return attachment.rel_path.split("/").pop() ?? "attachment";
}

/** Key identifying one attachment on one message (see {@link MediaItem.key}). */
export function mediaKey(message: Pick<MessageDto, "guid" | "ts_unix_ms">, attachment: Pick<AttachmentDto, "rel_path">, index: number): string {
  return `${message.guid ?? message.ts_unix_ms}:${index}:${attachment.rel_path}`;
}

/** Every photo and video in the conversation, in order - what the viewer's prev/next steps through. */
export function collectViewableMedia(messages: MessageDto[]): MediaItem[] {
  const items: MediaItem[] = [];
  for (const message of messages) {
    message.attachments.forEach((attachment, index) => {
      const kind = mediaKind(attachment.mime_type);
      if (kind !== "image" && kind !== "video") return;
      items.push({
        key: mediaKey(message, attachment, index),
        relPath: attachment.rel_path,
        kind,
        filename: attachmentFilename(attachment),
        tsUnixMs: message.ts_unix_ms,
        sender: message.sender_identifier,
        isFromMe: message.is_from_me,
      });
    });
  }
  return items;
}

const ILLEGAL_FILENAME_CHARS = /[<>:"/\\|?*\u0000-\u001f]/g;

/** A conversation title made safe to use as a file name on Windows, macOS and Linux. */
export function safeFileName(name: string, fallback = "conversation"): string {
  const cleaned = name.replace(ILLEGAL_FILENAME_CHARS, "_").replace(/\s+/g, " ").trim();
  // Windows also rejects names ending in a dot or space.
  const trimmed = cleaned.replace(/[. ]+$/, "").slice(0, 120);
  return trimmed || fallback;
}
