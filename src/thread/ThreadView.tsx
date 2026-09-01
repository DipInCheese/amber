// SPDX-License-Identifier: GPL-3.0-or-later
import { useVirtualizer } from "@tanstack/react-virtual";
import { forwardRef, useEffect, useImperativeHandle, useMemo, useRef } from "react";
import type { MessageDto } from "../lib/types";
import { DateSeparator } from "./DateSeparator";
import { MessageBubble } from "./MessageBubble";
import { buildThreadRows, findRowIndexForTimestamp, rowTimestamp } from "./rows";

export interface ThreadViewHandle {
  /** Scrolls so the first row at or after `tsUnixMs` is at the top. Used by the timeline scrubber. */
  scrollToDate: (tsUnixMs: number) => void;
}

export const ThreadView = forwardRef<
  ThreadViewHandle,
  {
    messages: MessageDto[];
    isGroup: boolean;
    /** Called with the timestamp of the topmost visible row whenever it changes, so the scrubber can track scroll position. */
    onVisibleDateChange?: (tsUnixMs: number) => void;
  }
>(function ThreadView({ messages, isGroup, onVisibleDateChange }, ref) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const rows = useMemo(() => buildThreadRows(messages), [messages]);

  const virtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => scrollRef.current,
    // Rough guess; measureElement (below) corrects it per-row for bubbles
    // that wrap text or carry attachments.
    estimateSize: (index) => (rows[index].kind === "date" ? 36 : 64),
    overscan: 12,
    getItemKey: (index) => rows[index].key,
  });

  useImperativeHandle(
    ref,
    () => ({
      scrollToDate: (tsUnixMs: number) => {
        const index = findRowIndexForTimestamp(rows, tsUnixMs);
        virtualizer.scrollToIndex(index, { align: "start" });
      },
    }),
    [rows, virtualizer],
  );

  // `range` is the actually-visible index span, distinct from
  // `getVirtualItems()` which also includes the overscan padding - using
  // the latter would report the overscanned-above row as "visible" and
  // make the scrubber's position indicator lag behind the real scroll.
  const topVisibleIndex = virtualizer.range?.startIndex;

  useEffect(() => {
    if (topVisibleIndex === undefined || !onVisibleDateChange) return;
    const row = rows[topVisibleIndex];
    if (row) onVisibleDateChange(rowTimestamp(row));
  }, [topVisibleIndex, rows, onVisibleDateChange]);

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
});
