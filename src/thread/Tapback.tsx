// SPDX-License-Identifier: GPL-3.0-or-later
import type { ReactionDto } from "../lib/types";

const svg = {
  viewBox: "0 0 24 24",
  width: 14,
  height: 14,
  fill: "currentColor",
  "aria-hidden": true,
} as const;

/** The small mark inside a reaction pill: drawn icons for the standard tapbacks, the emoji itself for custom ones. */
export function Tapback({ reaction }: { reaction: ReactionDto }) {
  const kind = reaction.kind.toLowerCase();
  if (kind.startsWith("love")) {
    return (
      <svg {...svg}>
        <path d="M12 20.5s-8-4.7-8-10.3A4.4 4.4 0 0 1 12 7.6a4.4 4.4 0 0 1 8 2.6c0 5.6-8 10.3-8 10.3Z" />
      </svg>
    );
  }
  if (kind.startsWith("dislike")) {
    return (
      <svg {...svg} style={{ transform: "scaleY(-1)" }}>
        <path d="M2.5 10.5h3.8v10H2.5zM8 10.5l3.4-7a2.1 2.1 0 0 1 3.9 1.1l-.6 3.9h4.7a2 2 0 0 1 2 2.4l-1.4 7a2 2 0 0 1-2 1.6H8z" />
      </svg>
    );
  }
  if (kind.startsWith("like")) {
    return (
      <svg {...svg}>
        <path d="M2.5 10.5h3.8v10H2.5zM8 10.5l3.4-7a2.1 2.1 0 0 1 3.9 1.1l-.6 3.9h4.7a2 2 0 0 1 2 2.4l-1.4 7a2 2 0 0 1-2 1.6H8z" />
      </svg>
    );
  }
  if (kind.startsWith("laugh")) return <span className="tapback-text">HA</span>;
  if (kind.startsWith("emphasi")) return <span className="tapback-text">!!</span>;
  if (kind.startsWith("question")) return <span className="tapback-text">?</span>;
  return <span className="tapback-text">{reaction.emoji ?? reaction.kind}</span>;
}
