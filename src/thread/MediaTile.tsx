// SPDX-License-Identifier: GPL-3.0-or-later
import { useCallback, useRef, useState, type CSSProperties } from "react";
import { attachmentUrl } from "../lib/api";
import { attachmentFilename, mediaFrameSize, mediaKind } from "../lib/media";
import type { AttachmentDto } from "../lib/types";
import { useContextMenu } from "../ui/ContextMenu";
import { IconDownload, IconExpand, IconFile } from "../ui/icons";
import { useMedia } from "./MediaContext";

/**
 * One photo or video, in a frame with the picture's own shape. The frame's
 * size comes from the archive's stored pixel size (not from the file once it
 * loads), so a row never changes height when its media arrives - that layout
 * shift is what makes a virtualized list jump while you scroll.
 */
export function MediaTile({
  attachment,
  mediaKey,
  variant,
  overflowCount = 0,
}: {
  attachment: AttachmentDto;
  mediaKey: string;
  /** `single`: sized by its aspect ratio. `cell`: fills a slot of the multi-photo grid. */
  variant: "single" | "cell";
  /** Extra photos hidden behind this tile in the grid ("+3"). */
  overflowCount?: number;
}) {
  const { open, save, scrubbing } = useMedia();
  const { show } = useContextMenu();
  const kind = mediaKind(attachment.mime_type);
  const filename = attachmentFilename(attachment);
  const url = attachmentUrl(attachment.rel_path);
  const frameRef = useRef<HTMLDivElement>(null);
  const [loaded, setLoaded] = useState(false);
  // The archive keeps the original bytes whatever the format, but the
  // webview can only *show* what its own decoders support. When it can't,
  // say so and keep the file reachable instead of leaving a broken image.
  const [cannotPreview, setCannotPreview] = useState(false);

  // An image that was already cached may finish before React attaches onLoad.
  const imageRef = useCallback((img: HTMLImageElement | null) => {
    if (img?.complete && img.naturalWidth > 0) setLoaded(true);
  }, []);

  const size = variant === "single" ? mediaFrameSize(attachment.width, attachment.height) : null;
  const style: CSSProperties | undefined =
    variant === "single"
      ? size
        ? { width: size.width, aspectRatio: `${size.width} / ${size.height}` }
        : { width: 260, aspectRatio: "4 / 3" }
      : undefined;

  const openViewer = () => open(mediaKey, frameRef.current);

  return (
    <div
      ref={frameRef}
      className={`media-tile media-${variant}${loaded ? " is-loaded" : ""}`}
      style={style}
      onContextMenu={(event) =>
        show(event, [
          { label: "Open", icon: <IconExpand size={16} />, onSelect: openViewer },
          { label: "Save…", icon: <IconDownload size={16} />, onSelect: () => save(attachment.rel_path, filename) },
        ])
      }
    >
      {cannotPreview ? (
        <div className="media-fallback">
          <IconFile size={24} />
          <span className="media-fallback-name">{filename}</span>
          <span className="note">Can’t be previewed here</span>
          <button type="button" className="btn btn-sm" onClick={() => save(attachment.rel_path, filename)}>
            <IconDownload size={14} />
            Save
          </button>
        </div>
      ) : scrubbing ? (
        <div className="media-placeholder skeleton" aria-hidden="true" />
      ) : kind === "image" ? (
        <button type="button" className="media-open" aria-label={`Open ${filename}`} onClick={openViewer}>
          <img
            ref={imageRef}
            className="media-img"
            src={url}
            alt={filename}
            loading="lazy"
            decoding="async"
            draggable={false}
            onLoad={() => setLoaded(true)}
            onError={() => setCannotPreview(true)}
          />
        </button>
      ) : (
        // eslint-disable-next-line jsx-a11y/media-has-caption
        <video
          className="media-img media-video"
          src={url}
          controls={variant === "single"}
          preload="metadata"
          onLoadedMetadata={() => setLoaded(true)}
          onClick={variant === "cell" ? openViewer : undefined}
          onError={() => setCannotPreview(true)}
        />
      )}

      {overflowCount > 0 && (
        <button type="button" className="media-more" aria-label={`Show all ${overflowCount + 4} photos`} onClick={openViewer}>
          +{overflowCount}
        </button>
      )}

      {!cannotPreview && variant === "single" && (
        <div className="media-tools">
          {kind === "video" && (
            <button type="button" className="media-tool" aria-label={`Expand ${filename}`} title="Expand" onClick={openViewer}>
              <IconExpand size={16} />
            </button>
          )}
          <button
            type="button"
            className="media-tool"
            aria-label={`Save ${filename}`}
            title="Save"
            onClick={() => save(attachment.rel_path, filename)}
          >
            <IconDownload size={16} />
          </button>
        </div>
      )}
    </div>
  );
}
