// SPDX-License-Identifier: GPL-3.0-or-later
import { useEffect, useRef } from "react";
import { IconChevronDown, IconChevronUp, IconSearch } from "../ui/icons";

/** The find bar: type to search the open conversation, Enter / Shift+Enter to step, Esc to leave. */
export function FindBar({
  query,
  onQueryChange,
  matchCount,
  position,
  onNext,
  onPrevious,
  onClose,
}: {
  query: string;
  onQueryChange: (query: string) => void;
  matchCount: number;
  /** 0-based position of the current match, or -1 when there is none. */
  position: number;
  onNext: () => void;
  onPrevious: () => void;
  onClose: () => void;
}) {
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    inputRef.current?.focus();
    inputRef.current?.select();
  }, []);

  const hasQuery = query.trim().length > 0;
  const status = !hasQuery
    ? ""
    : matchCount === 0
      ? "No results"
      : `${(position + 1).toLocaleString()} of ${matchCount.toLocaleString()}`;

  return (
    <div className="find-bar" role="search">
      <label className="search-field find-field">
        <IconSearch size={16} />
        <input
          ref={inputRef}
          type="search"
          placeholder="Find in conversation"
          aria-label="Find in conversation"
          value={query}
          onChange={(event) => onQueryChange(event.currentTarget.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              event.preventDefault();
              if (event.shiftKey) onPrevious();
              else onNext();
            } else if (event.key === "Escape") {
              event.preventDefault();
              event.stopPropagation();
              onClose();
            }
          }}
        />
      </label>
      <span className="find-status" aria-live="polite">
        {status}
      </span>
      <button type="button" className="btn btn-icon" aria-label="Previous result" disabled={matchCount === 0} onClick={onPrevious}>
        <IconChevronUp size={16} />
      </button>
      <button type="button" className="btn btn-icon" aria-label="Next result" disabled={matchCount === 0} onClick={onNext}>
        <IconChevronDown size={16} />
      </button>
      <button type="button" className="btn btn-sm" onClick={onClose}>
        Done
      </button>
    </div>
  );
}
