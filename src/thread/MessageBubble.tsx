// SPDX-License-Identifier: GPL-3.0-or-later
import { attachmentUrl } from "../lib/api";
import type { AttachmentDto, MessageDto } from "../lib/types";
import { formatTime } from "./formatting";
import { reactionGlyph } from "./reactionIcon";

function AttachmentView({ attachment }: { attachment: AttachmentDto }) {
  const url = attachmentUrl(attachment.rel_path);
  const mime = attachment.mime_type ?? "";

  if (mime.startsWith("image/")) {
    return (
      <img
        className="attachment attachment-image"
        src={url}
        alt={attachment.filename ?? "attachment"}
        loading="lazy"
        width={attachment.width ?? undefined}
        height={attachment.height ?? undefined}
      />
    );
  }

  if (mime.startsWith("video/")) {
    return (
      // eslint-disable-next-line jsx-a11y/media-has-caption
      <video className="attachment attachment-video" src={url} controls preload="metadata" />
    );
  }

  if (mime.startsWith("audio/")) {
    // eslint-disable-next-line jsx-a11y/media-has-caption
    return <audio className="attachment attachment-audio" src={url} controls preload="metadata" />;
  }

  return (
    <a className="attachment attachment-file" href={url} target="_blank" rel="noreferrer">
      {attachment.filename ?? "Attachment"}
    </a>
  );
}

export function MessageBubble({
  message,
  replyTo,
  showSender,
}: {
  message: MessageDto;
  replyTo: MessageDto | null;
  showSender: boolean;
}) {
  const side = message.is_from_me ? "from-me" : "from-them";

  if (message.is_unsent) {
    return (
      <div className={`message-row ${side}`}>
        <div className="bubble bubble-unsent">This message was unsent.</div>
      </div>
    );
  }

  return (
    <div className={`message-row ${side}`}>
      {showSender && !message.is_from_me && (
        <div className="sender-label">{message.sender_identifier ?? "Unknown"}</div>
      )}

      <div className="bubble-stack">
        {replyTo && (
          <div className="reply-quote">
            <span className="reply-quote-sender">
              {replyTo.is_from_me ? "You" : (replyTo.sender_identifier ?? "Unknown")}
            </span>
            <span className="reply-quote-text">{replyTo.text ?? "…"}</span>
          </div>
        )}

        <div className="bubble">
          {message.attachments.map((attachment) => (
            <AttachmentView key={attachment.rel_path} attachment={attachment} />
          ))}
          {message.text && <div className="bubble-text">{message.text}</div>}
          {message.is_edited && <div className="bubble-edited">Edited</div>}
        </div>

        {message.reactions.length > 0 && (
          <div className="reactions">
            {message.reactions
              .filter((r) => !r.is_removed)
              .map((reaction, i) => (
                <span
                  className="reaction-pill"
                  key={`${reaction.participant_identifier ?? "?"}-${reaction.kind}-${i}`}
                  title={`${reaction.kind}${
                    reaction.participant_identifier ? ` from ${reaction.participant_identifier}` : ""
                  }`}
                >
                  {reactionGlyph(reaction)}
                </span>
              ))}
          </div>
        )}

        <div className="bubble-timestamp">{formatTime(message.ts_unix_ms)}</div>
      </div>
    </div>
  );
}
