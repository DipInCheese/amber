// SPDX-License-Identifier: GPL-3.0-or-later
import { describe, expect, it } from "vitest";
import { attachmentFilename, collectViewableMedia, mediaKind, safeFileName } from "./media";
import type { AttachmentDto, MessageDto } from "./types";

function attachment(rel_path: string, mime_type: string | null, filename: string | null = null): AttachmentDto {
  return { rel_path, mime_type, filename, width: null, height: null, duration_ms: null };
}

function message(guid: string, ts: number, attachments: AttachmentDto[]): MessageDto {
  return {
    guid,
    ts_unix_ms: ts,
    sender_identifier: "+15551234567",
    is_from_me: false,
    text: null,
    reply_to_guid: null,
    is_edited: false,
    is_unsent: false,
    edit_history: null,
    attachments,
    reactions: [],
  };
}

describe("mediaKind", () => {
  it("classifies by MIME type", () => {
    expect(mediaKind("image/heic")).toBe("image");
    expect(mediaKind("video/quicktime")).toBe("video");
    expect(mediaKind("audio/mp4")).toBe("audio");
    expect(mediaKind("application/pdf")).toBe("file");
    expect(mediaKind(null)).toBe("file");
  });
});

describe("collectViewableMedia", () => {
  it("lists photos and videos in order, skipping audio and files", () => {
    const items = collectViewableMedia([
      message("a", 1, [attachment("attachments/aa/1.jpeg", "image/jpeg", "one.jpeg")]),
      message("b", 2, [attachment("attachments/bb/2.m4a", "audio/mp4"), attachment("attachments/cc/3.pdf", "application/pdf")]),
      message("c", 3, [attachment("attachments/dd/4.mov", "video/quicktime"), attachment("attachments/ee/5.png", "image/png")]),
    ]);

    expect(items.map((i) => i.relPath)).toEqual([
      "attachments/aa/1.jpeg",
      "attachments/dd/4.mov",
      "attachments/ee/5.png",
    ]);
    expect(items[0].filename).toBe("one.jpeg");
    expect(items[1].kind).toBe("video");
  });

  it("keeps the same file distinct when it is attached to two messages", () => {
    const same = attachment("attachments/aa/1.jpeg", "image/jpeg");
    const items = collectViewableMedia([message("a", 1, [same]), message("b", 2, [same])]);
    expect(items).toHaveLength(2);
    expect(new Set(items.map((i) => i.key)).size).toBe(2);
  });
});

describe("attachmentFilename", () => {
  it("prefers the real filename and falls back to the stored one", () => {
    expect(attachmentFilename({ filename: "IMG_0001.HEIC", rel_path: "attachments/ab/abc.heic" })).toBe("IMG_0001.HEIC");
    expect(attachmentFilename({ filename: null, rel_path: "attachments/ab/abc.heic" })).toBe("abc.heic");
  });
});

describe("safeFileName", () => {
  it("replaces characters Windows forbids", () => {
    expect(safeFileName('a/b\\c:d*e?f"g<h>i|j')).toBe("a_b_c_d_e_f_g_h_i_j");
  });

  it("keeps ordinary names, including plus signs and accents", () => {
    expect(safeFileName("+32468116958")).toBe("+32468116958");
    expect(safeFileName("Chloé & Zoë")).toBe("Chloé & Zoë");
  });

  it("never returns an empty or trailing-dot name", () => {
    expect(safeFileName("   ")).toBe("conversation");
    expect(safeFileName("???")).toBe("___");
    expect(safeFileName("name.. ")).toBe("name");
  });
});
