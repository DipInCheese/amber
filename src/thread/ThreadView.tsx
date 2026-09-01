// SPDX-License-Identifier: GPL-3.0-or-later
import { useVirtualizer } from "@tanstack/react-virtual";
import { useRef } from "react";
import type { MessageDto } from "../lib/types";
import { DateSeparator } from "./DateSeparator";
import { MessageBubble } from "./MessageBubble";
import { buildThreadRows } from "./rows";

export function ThreadView({
  messages,
  isGroup,
}: {
  messages: MessageDto[];
  isGroup: boolean;
}) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const rows = buildThreadRows(messages);

  const virtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => scrollRef.current,
    // Rough guess; measureElement (below) corrects it per-row for bubbles
    // that wrap text or carry attachments.
    estimateSize: (index) => (rows[index].kind === "date" ? 36 : 64),
    overscan: 12,
    getItemKey: (index) => rows[index].key,
  });

  return (
    <div className="thread-scroll" ref={scrollRef}>
      <div
        className="thread-inner"
        style={{ height: virtualizer.getTotalSize(), position: "relative" }}
      >
        {virtualizer.getVirtualItems().map((virtualRow) => {
          const row = rows[virtualRow.index];
          return (
            <div
              key={row.key}
              ref={virtualizer.measureElement}
              data-index={virtualRow.index}
              className="thread-row"
              style={{
                position: "absolute",
                top: 0,
                left: 0,
                width: "100%",
                transform: `translateY(${virtualRow.start}px)`,
              }}
            >
              {row.kind === "date" ? (
                <DateSeparator tsUnixMs={row.tsUnixMs} />
              ) : (
                <MessageBubble
                  message={row.message}
                  replyTo={row.replyTo}
                  showSender={isGroup}
                />
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
