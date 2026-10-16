// SPDX-License-Identifier: GPL-3.0-or-later
import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import { attachmentUrl } from "../lib/api";
import type { MediaItem } from "../lib/media";
import { usePeople } from "../thread/PeopleContext";
import { IconChevronLeft, IconChevronRight, IconClose, IconDownload } from "../ui/icons";

/** Where the clicked thumbnail was on screen, so the photo can grow out of it. */
export interface OriginRect {
  left: number;
  top: number;
  width: number;
  height: number;
}

const dateTimeFormatter = new Intl.DateTimeFormat(undefined, {
  dateStyle: "long",
  timeStyle: "short",
});

function captionFor(item: MediaItem, who: string): string {
  return `${who} · ${dateTimeFormatter.format(new Date(item.tsUnixMs))}`;
}

/**
 * Full-screen photo/video viewer: click a thumbnail to open, Esc or a click
 * on the backdrop to close, arrow keys (or the side buttons) to step through
 * every photo and video in the conversation, and a download button.
 */
export function Lightbox({
  items,
  index,
  origin,
  onIndexChange,
  onClose,
  onSave,
}: {
  items: MediaItem[];
  index: number;
  /** Only used for the opening animation of the first item shown. */
  origin: OriginRect | null;
  onIndexChange: (index: number) => void;
  onClose: () => void;
  onSave: (item: MediaItem) => void;
}) {
  const item = items[index];
  const { nameFor } = usePeople();
  const dialogRef = useRef<HTMLDivElement>(null);
  const imageRef = useRef<HTMLImageElement>(null);
  const growFromOrigin = useRef(origin);
  const [failed, setFailed] = useState(false);
  const [closing, setClosing] = useState(false);

  const hasPrev = index > 0;
  const hasNext = index < items.length - 1;

  const goTo = useCallback(
    (next: number) => {
      if (next < 0 || next >= items.length) return;
      growFromOrigin.current = null;
      setFailed(false);
      onIndexChange(next);
    },
    [items.length, onIndexChange],
  );

  const close = useCallback(() => {
    // Let the fade-out play before unmounting.
    setClosing(true);
    window.setTimeout(onClose, 140);
  }, [onClose]);

  // Keyboard control; focus the dialog so keys land here, give it back on close.
  useEffect(() => {
    const previouslyFocused = document.activeElement as HTMLElement | null;
    dialogRef.current?.focus();
    return () => previouslyFocused?.focus?.();
  }, []);

  const onKeyDown = (event: React.KeyboardEvent) => {
    if (event.key === "Escape") {
      event.preventDefault();
      close();
    } else if (event.key === "ArrowLeft") {
      event.preventDefault();
      goTo(index - 1);
    } else if (event.key === "ArrowRight") {
      event.preventDefault();
      goTo(index + 1);
    }
  };

  // Warm the neighbours so stepping is instant (HEIC photos need converting).
  useEffect(() => {
    for (const neighbour of [items[index - 1], items[index + 1]]) {
      if (neighbour?.kind === "image") new Image().src = attachmentUrl(neighbour.relPath);
    }
  }, [items, index]);

  // The photo grows out of the thumbnail you clicked (Web Animations API:
  // runs on the compositor, so it stays smooth even while the image decodes).
  const animateFromOrigin = () => {
    const from = growFromOrigin.current;
    const image = imageRef.current;
    growFromOrigin.current = null;
    if (!from || !image || window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;

    const to = image.getBoundingClientRect();
    if (to.width === 0 || to.height === 0) return;
    image.animate(
      [
        {
          transform: `translate(${from.left - to.left}px, ${from.top - to.top}px) scale(${
            from.width / to.width
          }, ${from.height / to.height})`,
          transformOrigin: "top left",
        },
        { transform: "none", transformOrigin: "top left" },
      ],
      { duration: 260, easing: "cubic-bezier(0.2, 0.8, 0.2, 1)" },
    );
  };

  useLayoutEffect(() => {
    setFailed(false);
  }, [item?.key]);

  if (!item) return null;
  const url = attachmentUrl(item.relPath);

  return (
    <div
      className={`lightbox${closing ? " lightbox-closing" : ""}`}
      role="dialog"
      aria-modal="true"
      aria-label="Photo viewer"
      tabIndex={-1}
      ref={dialogRef}
      onKeyDown={onKeyDown}
    >
      <div className="lightbox-backdrop" onClick={close} />

      <header className="lightbox-bar">
        <div className="lightbox-caption">
          <strong>{item.filename}</strong>
          <span>{captionFor(item, item.isFromMe ? "You" : nameFor(item.sender))}</span>
        </div>
        <div className="lightbox-actions">
          <span className="lightbox-counter">
            {(index + 1).toLocaleString()} / {items.length.toLocaleString()}
          </span>
          <button type="button" className="lightbox-button" onClick={() => onSave(item)}>
            <IconDownload size={16} />
            Download
          </button>
          <button type="button" className="lightbox-button lightbox-icon" aria-label="Close" onClick={close}>
            <IconClose size={18} />
          </button>
        </div>
      </header>

      {hasPrev && (
        <button
          type="button"
          className="lightbox-nav lightbox-prev"
          aria-label="Previous photo"
          onClick={() => goTo(index - 1)}
        >
          <IconChevronLeft size={26} />
        </button>
      )}
      {hasNext && (
        <button
          type="button"
          className="lightbox-nav lightbox-next"
          aria-label="Next photo"
          onClick={() => goTo(index + 1)}
        >
          <IconChevronRight size={26} />
        </button>
      )}

      <div
        className="lightbox-stage"
        onClick={(event) => {
          if (event.target === event.currentTarget) close();
        }}
      >
        {failed ? (
          <div className="lightbox-failed" key={item.key}>
            <p>This can’t be previewed here.</p>
            <button type="button" className="lightbox-button" onClick={() => onSave(item)}>
              <IconDownload size={16} />
              Download {item.filename}
            </button>
          </div>
        ) : item.kind === "image" ? (
          <img
            key={item.key}
            ref={imageRef}
            className="lightbox-media"
            src={url}
            alt={item.filename}
            draggable={false}
            onLoad={animateFromOrigin}
            onError={() => setFailed(true)}
          />
        ) : (
          // eslint-disable-next-line jsx-a11y/media-has-caption
          <video
            key={item.key}
            className="lightbox-media"
            src={url}
            controls
            autoPlay
            onError={() => setFailed(true)}
          />
        )}
      </div>
    </div>
  );
}
