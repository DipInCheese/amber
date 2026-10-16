// SPDX-License-Identifier: GPL-3.0-or-later
import { memo } from "react";
import { mediaKey, mediaKind } from "../lib/media";
import type { AttachmentDto, MessageDto } from "../lib/types";
import { splitByQuery } from "../search/search";
import { useSearch } from "../search/SearchContext";
import { Avatar } from "../ui/Avatar";
import { useContextMenu } from "../ui/ContextMenu";
import { AttachmentView } from "./AttachmentView";
import { formatTime } from "./formatting";
import { MediaTile } from "./MediaTile";
import { usePeople } from "./PeopleContext";
import type { RunPosition } from "./rows";
import { Tapback } from "./Tapback";

const GRID_VISIBLE = 4;

function isGreenService(service: string | null): boolean {
  return service !== null && service !== "" && service.toLowerCase() !== "imessage";
}

function Text({ text, query, active }: { text: string; query: string; active: boolean }) {
  if (!query) return <>{text}</>;
  return (
    <>
      {splitByQuery(text, query).map((part, index) =>
        part.match ? (
          <mark key={index} className={active ? "find-mark is-active" : "find-mark"}>
            {part.text}
          </mark>
        ) : (
          part.text
        ),
      )}
    </>
  );
}

export const MessageBubble = memo(function MessageBubble({
  message,
  messageKey,
  replyTo,
  position,
  timeAbove,
}: {
  message: MessageDto;
  messageKey: string;
  replyTo: MessageDto | null;
  position: RunPosition;
  timeAbove: number | null;
}) {
  const { nameFor, isGroup } = usePeople();
  const { query, activeKey } = useSearch();
  const { show } = useContextMenu();

  const side = message.is_from_me ? "from-me" : "from-them";
  const sms = isGreenService(message.service);
  const startsRun = position === "single" || position === "first";
  const endsRun = position === "single" || position === "last";
  const senderName = nameFor(message.sender_identifier);

  const indexed = message.attachments.map((attachment, index) => ({ attachment, index }));
  const isMedia = ({ attachment }: { attachment: AttachmentDto }) => {
    const kind = mediaKind(attachment.mime_type);
    return kind === "image" || kind === "video";
  };
  const media = indexed.filter(isMedia);
  const others = indexed.filter((item) => !isMedia(item));
  // Photos and videos sit on their own, with any text in a bubble beneath, as in Messages.
  const hasBubble = message.is_unsent || Boolean(message.text) || others.length > 0;

  const copyText = () => void navigator.clipboard?.writeText(message.text ?? "").catch(() => {});

  const classes = [
    "message-row",
    side,
    `pos-${position}`,
    sms ? "is-sms" : "",
    isGroup && !message.is_from_me ? "has-avatar-slot" : "",
    timeAbove !== null ? "has-time" : "",
  ]
    .filter(Boolean)
    .join(" ");

  const active = activeKey === messageKey;

  return (
    <div className={classes}>
      {timeAbove !== null && <div className="time-stamp">{formatTime(timeAbove)}</div>}

      {isGroup && !message.is_from_me && startsRun && <div className="sender-label">{senderName}</div>}

      <div className="bubble-line">
        {isGroup && !message.is_from_me && (
          <div className="avatar-slot">{endsRun && <Avatar name={senderName} size={28} />}</div>
        )}

        <div className="bubble-stack">
          {replyTo && (
            <div className="reply-quote">
              <span className="reply-quote-sender">
                {replyTo.is_from_me ? "You" : nameFor(replyTo.sender_identifier)}
              </span>
              <span className="reply-quote-text">{replyTo.text ?? "…"}</span>
            </div>
          )}

          {!message.is_unsent && media.length === 1 && (
            <MediaTile
              attachment={media[0].attachment}
              mediaKey={mediaKey(message, media[0].attachment, media[0].index)}
              variant="single"
            />
          )}
          {!message.is_unsent && media.length > 1 && (
            <div className={`media-grid media-grid-${Math.min(media.length, GRID_VISIBLE)}`}>
              {media.slice(0, GRID_VISIBLE).map(({ attachment, index }, slot) => (
                <MediaTile
                  key={`${index}:${attachment.rel_path}`}
                  attachment={attachment}
                  mediaKey={mediaKey(message, attachment, index)}
                  variant="cell"
                  overflowCount={slot === GRID_VISIBLE - 1 ? media.length - GRID_VISIBLE : 0}
                />
              ))}
            </div>
          )}

          {hasBubble && (
          <div
            className={`bubble${message.is_unsent ? " bubble-unsent" : ""}`}
            onContextMenu={(event) => {
              if (!message.text || window.getSelection()?.toString()) return;
              show(event, [{ label: "Copy", onSelect: copyText }]);
            }}
          >
            {message.is_unsent ? (
              "This message was unsent."
            ) : (
              <>
                {others.map(({ attachment, index }) => (
                  <AttachmentView key={`${index}:${attachment.rel_path}`} attachment={attachment} />
                ))}
                {message.text && (
                  <div className="bubble-text">
                    <Text text={message.text} query={query} active={active} />
                  </div>
                )}
                {message.is_edited && <div className="bubble-edited">Edited</div>}
              </>
            )}
          </div>
          )}

          {message.reactions.some((r) => !r.is_removed) && (
            <div className="reactions">
              {message.reactions
                .filter((r) => !r.is_removed)
                .map((reaction, i) => (
                  <span
                    className="reaction-pill"
                    key={`${reaction.participant_identifier ?? "?"}-${reaction.kind}-${i}`}
                    title={`${reaction.kind}${
                      reaction.participant_identifier ? ` from ${nameFor(reaction.participant_identifier)}` : ""
                    }`}
                  >
                    <Tapback reaction={reaction} />
                  </span>
                ))}
            </div>
          )}
        </div>

        <span className="hover-time">{formatTime(message.ts_unix_ms)}</span>
      </div>
    </div>
  );
});
