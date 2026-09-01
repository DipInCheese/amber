// SPDX-License-Identifier: GPL-3.0-or-later
import { useCallback, useRef, useState } from "react";
import type { DayBucketDto, MessageDto } from "./lib/types";
import { ThreadView, type ThreadViewHandle } from "./thread/ThreadView";
import { TimelineScrubber } from "./timeline/TimelineScrubber";

/** The main list + scrubber, kept in sync: dragging the scrubber jumps the list, scrolling the list moves the scrubber's position indicator. */
export function ConversationView({
  messages,
  dayBuckets,
  isGroup,
}: {
  messages: MessageDto[];
  dayBuckets: DayBucketDto[];
  isGroup: boolean;
}) {
  const threadRef = useRef<ThreadViewHandle>(null);
  const [currentTsUnixMs, setCurrentTsUnixMs] = useState<number | null>(
    messages[0]?.ts_unix_ms ?? null,
  );

  const handleScrub = useCallback((tsUnixMs: number) => {
    threadRef.current?.scrollToDate(tsUnixMs);
  }, []);

  return (
    <div className="conversation-view">
      <ThreadView
        ref={threadRef}
        messages={messages}
        isGroup={isGroup}
        onVisibleDateChange={setCurrentTsUnixMs}
      />
      <TimelineScrubber
        dayBuckets={dayBuckets}
        currentTsUnixMs={currentTsUnixMs}
        onScrub={handleScrub}
      />
    </div>
  );
}
