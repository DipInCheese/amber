// SPDX-License-Identifier: GPL-3.0-or-later
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Lightbox, type OriginRect } from "./lightbox/Lightbox";
import { attachmentFilename, collectViewableMedia, type MediaItem } from "./lib/media";
import { saveAttachmentWithDialog } from "./lib/saveFile";
import type { DayBucketDto, MessageDto, ParticipantDto } from "./lib/types";
import { findMatches } from "./search/search";
import { FindBar } from "./search/FindBar";
import { SearchContext, type SearchState } from "./search/SearchContext";
import { MediaContext, type MediaActions } from "./thread/MediaContext";
import { PeopleContext, type People } from "./thread/PeopleContext";
import { messageRowKey } from "./thread/rows";
import { ThreadView, type ThreadViewHandle } from "./thread/ThreadView";
import { TimelineScrubber } from "./timeline/TimelineScrubber";
import { Toast, type ToastMessage } from "./Toast";

/** The main list + scrubber, kept in sync: dragging the scrubber jumps the list, scrolling the list moves the scrubber's position indicator. */
export function ConversationView({
  messages,
  dayBuckets,
  isGroup,
  participants,
  findOpen,
  onFindClose,
}: {
  messages: MessageDto[];
  dayBuckets: DayBucketDto[];
  isGroup: boolean;
  participants: ParticipantDto[];
  findOpen: boolean;
  onFindClose: () => void;
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

  const people = useMemo<People>(() => {
    const names = new Map<string, string>();
    for (const participant of participants) {
      if (!participant.is_me && participant.display_name) names.set(participant.identifier, participant.display_name);
    }
    return { isGroup, nameFor: (identifier) => (identifier ? (names.get(identifier) ?? identifier) : "Unknown") };
  }, [participants, isGroup]);

  // --- Find ---------------------------------------------------------------
  const [query, setQuery] = useState("");
  const [matchPosition, setMatchPosition] = useState(0);
  const matches = useMemo(() => findMatches(messages, query), [messages, query]);
  const matchKeys = useMemo(() => matches.map((i) => messageRowKey(messages[i], i)), [matches, messages]);
  const currentTsRef = useRef(currentTsUnixMs);
  useEffect(() => {
    currentTsRef.current = currentTsUnixMs;
  });

  // A new search starts at the first result from where you are reading.
  useEffect(() => {
    if (matches.length === 0) {
      setMatchPosition(0);
      return;
    }
    const from = currentTsRef.current ?? 0;
    const first = matches.findIndex((i) => messages[i].ts_unix_ms >= from);
    setMatchPosition(first === -1 ? matches.length - 1 : first);
  }, [matches, messages]);

  const activeKey = findOpen && matchKeys.length > 0 ? (matchKeys[matchPosition] ?? null) : null;
  useEffect(() => {
    if (activeKey) threadRef.current?.scrollToMessageKey(activeKey);
  }, [activeKey]);

  const step = useCallback(
    (direction: 1 | -1) => {
      if (matches.length === 0) return;
      setMatchPosition((position) => (position + direction + matches.length) % matches.length);
    },
    [matches.length],
  );

  const closeFind = useCallback(() => {
    setQuery("");
    onFindClose();
  }, [onFindClose]);

  const searchState = useMemo<SearchState>(
    () => ({ query: findOpen ? query : "", activeKey }),
    [findOpen, query, activeKey],
  );

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
        showToast(`Couldn’t save ${filename}: ${String(err)}`, "error");
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
    <PeopleContext.Provider value={people}>
      <SearchContext.Provider value={searchState}>
        <MediaContext.Provider value={mediaActions}>
          <div className="conversation-view">
            {findOpen && (
              <FindBar
                query={query}
                onQueryChange={setQuery}
                matchCount={matches.length}
                position={matches.length === 0 ? -1 : matchPosition}
                onNext={() => step(1)}
                onPrevious={() => step(-1)}
                onClose={closeFind}
              />
            )}
            <ThreadView
              ref={threadRef}
              messages={messages}
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
      </SearchContext.Provider>
    </PeopleContext.Provider>
  );
}
