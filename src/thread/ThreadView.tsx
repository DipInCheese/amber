// SPDX-License-Identifier: GPL-3.0-or-later
import { useVirtualizer } from "@tanstack/react-virtual";
import { forwardRef, useCallback, useEffect, useImperativeHandle, useMemo, useRef } from "react";
import type { MessageDto } from "../lib/types";
import { DateSeparator } from "./DateSeparator";
import { MessageBubble } from "./MessageBubble";
import { buildThreadRows, findRowIndexForTimestamp, rowTimestamp } from "./rows";
import { easeToward, prefersReducedMotion } from "./smoothScroll";

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
    /** True while the thread is still gliding toward a timeline target - media stays light until it lands. */
    onScrollAnimatingChange?: (animating: boolean) => void;
  }
>(function ThreadView({ messages, isGroup, onVisibleDateChange, onScrollAnimatingChange }, ref) {
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

  // --- Gliding to a timeline target --------------------------------------
  // Jumping straight to the target reads as a teleport, and the browser's
  // own smooth scrolling gives up on distances like "three years of
  // messages". Instead the thread chases the target: every frame it closes a
  // fixed fraction of the remaining gap (see smoothScroll.ts). The target is
  // re-read from the virtualizer each frame, because row heights are only
  // estimates until rows have been measured and the offset shifts as it
  // gets closer.
  const targetIndexRef = useRef<number | null>(null);
  const frameRef = useRef<number | null>(null);
  const lastFrameTimeRef = useRef(0);
  const settledFramesRef = useRef(0);
  const animatingChangeRef = useRef(onScrollAnimatingChange);
  useEffect(() => {
    animatingChangeRef.current = onScrollAnimatingChange;
  });

  const stopChase = useCallback(() => {
    if (frameRef.current !== null) cancelAnimationFrame(frameRef.current);
    const wasAnimating = frameRef.current !== null || targetIndexRef.current !== null;
    frameRef.current = null;
    targetIndexRef.current = null;
    if (wasAnimating) animatingChangeRef.current?.(false);
  }, []);

  const chaseStep = useCallback(
    (now: number) => {
      const element = scrollRef.current;
      const index = targetIndexRef.current;
      const target = index === null ? undefined : virtualizer.getOffsetForIndex(index, "start")?.[0];
      if (!element || target === undefined) {
        stopChase();
        return;
      }

      const clamped = Math.min(Math.max(0, target), element.scrollHeight - element.clientHeight);
      const dt = Math.min(50, Math.max(1, now - lastFrameTimeRef.current));
      lastFrameTimeRef.current = now;

      const next = easeToward(element.scrollTop, clamped, dt);
      element.scrollTop = next;

      // Rows measured on arrival can nudge the target a little, so only call
      // it finished after two consecutive frames sitting on it.
      settledFramesRef.current = next === clamped ? settledFramesRef.current + 1 : 0;
      if (settledFramesRef.current >= 2) {
        stopChase();
        return;
      }
      frameRef.current = requestAnimationFrame(chaseStep);
    },
    [stopChase, virtualizer],
  );

  const glideToIndex = useCallback(
    (index: number) => {
      if (prefersReducedMotion()) {
        virtualizer.scrollToIndex(index, { align: "start" });
        return;
      }
      const wasIdle = frameRef.current === null;
      targetIndexRef.current = index;
      settledFramesRef.current = 0;
      if (wasIdle) {
        lastFrameTimeRef.current = performance.now();
        animatingChangeRef.current?.(true);
        frameRef.current = requestAnimationFrame(chaseStep);
      }
    },
    [chaseStep, virtualizer],
  );

  // The user taking over (wheel, touch, keys, clicking in the thread) always
  // wins over an animation in flight.
  useEffect(() => {
    const element = scrollRef.current;
    if (!element) return;
    const events = ["wheel", "touchstart", "pointerdown", "keydown"] as const;
    for (const name of events) element.addEventListener(name, stopChase, { passive: true });
    return () => {
      for (const name of events) element.removeEventListener(name, stopChase);
    };
  }, [stopChase]);

  // A different conversation (or reload) invalidates any target index.
  useEffect(() => stopChase, [rows, stopChase]);

  useImperativeHandle(
    ref,
    () => ({
      scrollToDate: (tsUnixMs: number) => {
        glideToIndex(findRowIndexForTimestamp(rows, tsUnixMs));
      },
    }),
    [rows, glideToIndex],
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
    <div className="thread-scroll" ref={scrollRef} tabIndex={0} aria-label="Messages">
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
