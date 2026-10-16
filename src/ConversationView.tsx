// SPDX-License-Identifier: GPL-3.0-or-later
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Lightbox, type OriginRect } from "./lightbox/Lightbox";
import { attachmentFilename, collectViewableMedia, type MediaItem } from "./lib/media";
import { saveAttachmentWithDialog } from "./lib/saveFile";
import type { DayBucketDto, MessageDto } from "./lib/types";
import { MediaContext, type MediaActions } from "./thread/MediaContext";
import { ThreadView, type ThreadViewHandle } from "./thread/ThreadView";
import { TimelineScrubber } from "./timeline/TimelineScrubber";
import { Toast, type ToastMessage } from "./Toast";

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
  const [scrubbing, setScrubbing] = useState(false);
  // The thread keeps gliding for a moment after the finger lifts; media stays
  // light until it has actually landed.
  const [gliding, setGliding] = useState(false);
  const [lightbox, setLightbox] = useState<{ index: number; origin: OriginRect | null } | null>(null);
  const [toast, setToast] = useState<ToastMessage | null>(null);
  const toastTimer = useRef<number | undefined>(undefined);
  const toastId = useRef(0);

  const media = useMemo(() => collectViewableMedia(messages), [messages]);
  const indexByKey = useMemo(() => new Map(media.map((item, i) => [item.key, i])), [media]);

  useEffect(() => () => window.clearTimeout(toastTimer.current), []);

  const showToast = useCallback((text: string, kind: ToastMessage["kind"]) => {
    window.clearTimeout(toastTimer.current);
    toastId.current += 1;
    setToast({ id: toastId.current, text, kind });
    toastTimer.current = window.setTimeout(() => setToast(null), 3500);
  }, []);

  const save = useCallback(
    async (relPath: string, filename: string) => {
      try {
        const result = await saveAttachmentWithDialog(relPath, filename);
        if (result.status === "saved") showToast(`Saved ${filename}`, "info");
      } catch (err) {
        showToast(`Couldn't save ${filename}: ${String(err)}`, "error");
      }
    },
    [showToast],
  );

  const open = useCallback(
    (key: string, origin: Element | null) => {
      const index = indexByKey.get(key);
      if (index === undefined) return;
      const rect = origin?.getBoundingClientRect();
      setLightbox({
        index,
        origin: rect ? { left: rect.left, top: rect.top, width: rect.width, height: rect.height } : null,
      });
    },
    [indexByKey],
  );

  const mediaActions = useMemo<MediaActions>(
    () => ({ open, save, scrubbing: scrubbing || gliding }),
    [open, save, scrubbing, gliding],
  );

  const handleScrub = useCallback((tsUnixMs: number) => {
    threadRef.current?.scrollToDate(tsUnixMs);
  }, []);

  const saveItem = useCallback(
    (item: MediaItem) => void save(item.relPath, item.filename || attachmentFilename({ filename: null, rel_path: item.relPath })),
    [save],
  );

  return (
    <MediaContext.Provider value={mediaActions}>
      <div className="conversation-view">
        <ThreadView
          ref={threadRef}
          messages={messages}
          isGroup={isGroup}
          onVisibleDateChange={setCurrentTsUnixMs}
          onScrollAnimatingChange={setGliding}
        />
        <TimelineScrubber
          dayBuckets={dayBuckets}
          currentTsUnixMs={currentTsUnixMs}
          onScrub={handleScrub}
          onScrubbingChange={setScrubbing}
        />
      </div>

      {lightbox && (
        <Lightbox
          items={media}
          index={lightbox.index}
          origin={lightbox.origin}
          onIndexChange={(index) => setLightbox({ index, origin: null })}
          onClose={() => setLightbox(null)}
          onSave={saveItem}
        />
      )}

      <Toast toast={toast} />
    </MediaContext.Provider>
  );
}
