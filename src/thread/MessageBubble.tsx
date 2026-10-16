// SPDX-License-Identifier: GPL-3.0-or-later
import { memo } from "react";
import { mediaKey, mediaKind } from "../lib/media";
import type { MessageDto } from "../lib/types";
import { AttachmentView } from "./AttachmentView";
import { formatTime } from "./formatting";
import { reactionGlyph } from "./reactionIcon";

export const MessageBubble = memo(function MessageBubble({
  message,
  replyTo,
  showSender,
}: {
  message: MessageDto;
  replyTo: MessageDto | null;
  showSender: boolean;
}) {
  const side = message.is_from_me ? "from-me" : "from-them";
  // A photo/video sent on its own shows as just the picture, like Messages.
  const mediaOnly =
    !message.text &&
    message.attachments.length > 0 &&
    message.attachments.every((a) => mediaKind(a.mime_type) !== "file" && mediaKind(a.mime_type) !== "audio");

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

        <div className={`bubble${mediaOnly ? " bubble-media-only" : ""}`}>
          {message.attachments.map((attachment, index) => (
            <AttachmentView
              key={`${index}:${attachment.rel_path}`}
              attachment={attachment}
              mediaKey={mediaKey(message, attachment, index)}
            />
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
});
