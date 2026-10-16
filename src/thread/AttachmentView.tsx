// SPDX-License-Identifier: GPL-3.0-or-later
import { useRef, useState } from "react";
import { attachmentUrl } from "../lib/api";
import { attachmentFilename, mediaKind } from "../lib/media";
import type { AttachmentDto } from "../lib/types";
import { useMedia } from "./MediaContext";

function DownloadIcon() {
  return (
    <svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M12 4v11" />
      <path d="m7 11 5 5 5-5" />
      <path d="M5 20h14" />
    </svg>
  );
}

function ExpandIcon() {
  return (
    <svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M14 4h6v6" />
      <path d="M10 20H4v-6" />
      <path d="m20 4-7 7" />
      <path d="m4 20 7-7" />
    </svg>
  );
}

/**
 * One attachment in a bubble. Photos and videos sit in a frame of fixed
 * height, so a row's height never changes when its media finishes loading -
 * that layout shift is what makes a virtualized list jump while you scroll.
 */
export function AttachmentView({
  attachment,
  mediaKey,
}: {
  attachment: AttachmentDto;
  mediaKey: string;
}) {
  const { open, save, scrubbing } = useMedia();
  const kind = mediaKind(attachment.mime_type);
  const filename = attachmentFilename(attachment);
  const url = attachmentUrl(attachment.rel_path);
  const frameRef = useRef<HTMLDivElement>(null);
  // The archive keeps the original bytes whatever the format, but the
  // webview can only *show* what its own decoders support (iPhone photos
  // are often HEIC, videos HEVC). When it can't, say so and keep the file
  // reachable instead of leaving a broken image.
  const [cannotPreview, setCannotPreview] = useState(false);

  if ((kind === "image" || kind === "video") && !cannotPreview) {
    return (
      <div className="attachment-frame" ref={frameRef}>
        {scrubbing ? (
          <div className="attachment-placeholder" aria-hidden="true" />
        ) : kind === "image" ? (
          <button
            type="button"
            className="attachment-open"
            aria-label={`Open ${filename}`}
            onClick={(event) => open(mediaKey, event.currentTarget.querySelector("img"))}
          >
            <img
              className="attachment-media"
              src={url}
              alt={filename}
              loading="lazy"
              decoding="async"
              draggable={false}
              onError={() => setCannotPreview(true)}
            />
          </button>
        ) : (
          // eslint-disable-next-line jsx-a11y/media-has-caption
          <video
            className="attachment-media"
            src={url}
            controls
            preload="metadata"
            onError={() => setCannotPreview(true)}
          />
        )}

        <div className="attachment-tools">
          {kind === "video" && (
            <button
              type="button"
              className="attachment-tool"
              aria-label={`Expand ${filename}`}
              title="Expand"
              onClick={() => open(mediaKey, frameRef.current)}
            >
              <ExpandIcon />
            </button>
          )}
          <button
            type="button"
            className="attachment-tool"
            aria-label={`Save ${filename}`}
            title="Save"
            onClick={() => save(attachment.rel_path, filename)}
          >
            <DownloadIcon />
          </button>
        </div>
      </div>
    );
  }

  if (kind === "audio" && !cannotPreview) {
    // eslint-disable-next-line jsx-a11y/media-has-caption
    return (
      <audio
        className="attachment attachment-audio"
        src={url}
        controls
        preload="metadata"
        onError={() => setCannotPreview(true)}
      />
    );
  }

  return (
    <div className="attachment-file">
      <span className="attachment-file-name">
        {filename}
        {cannotPreview && <span className="attachment-note"> - can't be previewed here</span>}
      </span>
      <button
        type="button"
        className="attachment-file-save"
        aria-label={`Save ${filename}`}
        onClick={() => save(attachment.rel_path, filename)}
      >
        <DownloadIcon />
        Save
      </button>
    </div>
  );
}
