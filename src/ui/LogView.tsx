// SPDX-License-Identifier: GPL-3.0-or-later
import { useEffect, useRef } from "react";

/** How many lines of a live log are kept; older ones scroll away. */
export const LOG_LIMIT = 500;

/** Appends `line` to `lines`, keeping at most {@link LOG_LIMIT}. */
export function appendLog(lines: string[], line: string): string[] {
  const next = lines.length >= LOG_LIMIT ? lines.slice(lines.length - LOG_LIMIT + 1) : lines.slice();
  next.push(line);
  return next;
}

/** A live, auto-scrolling log of what a long task is doing. */
export function LogView({ lines, empty = "Waiting for the first message…" }: { lines: string[]; empty?: string }) {
  const ref = useRef<HTMLPreElement>(null);
  useEffect(() => {
    const el = ref.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [lines]);
  return (
    <pre ref={ref} className="log-view selectable" aria-label="Details" aria-live="off">
      {lines.length > 0 ? lines.join("\n") : empty}
    </pre>
  );
}
