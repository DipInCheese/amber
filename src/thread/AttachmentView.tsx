// SPDX-License-Identifier: GPL-3.0-or-later
import { useState } from "react";
import { attachmentUrl } from "../lib/api";
import { attachmentFilename, mediaKind } from "../lib/media";
import type { AttachmentDto } from "../lib/types";
import { IconDownload, IconFile } from "../ui/icons";
import { useMedia } from "./MediaContext";

/**
 * Everything in a bubble that isn't a photo or video: a voice note plays in
 * place, any other file shows its name with a Save button. (Photos and
 * videos are {@link MediaTile}s.)
 */
export function AttachmentView({ attachment }: { attachment: AttachmentDto }) {
  const { save } = useMedia();
  const kind = mediaKind(attachment.mime_type);
  const filename = attachmentFilename(attachment);
  const [cannotPreview, setCannotPreview] = useState(false);

  if (kind === "audio" && !cannotPreview) {
    return (
      // eslint-disable-next-line jsx-a11y/media-has-caption
      <audio
        className="attachment-audio"
        src={attachmentUrl(attachment.rel_path)}
        controls
        preload="metadata"
        onError={() => setCannotPreview(true)}
      />
    );
  }

  return (
    <div className="attachment-file">
      <IconFile size={18} />
      <span className="attachment-file-name">
        {filename}
        {cannotPreview && <span className="attachment-note"> - can’t be played here</span>}
      </span>
      <button
        type="button"
        className="attachment-file-save"
        aria-label={`Save ${filename}`}
        onClick={() => save(attachment.rel_path, filename)}
      >
        <IconDownload size={14} />
        Save
      </button>
    </div>
  );
}
