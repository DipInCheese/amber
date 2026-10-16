// SPDX-License-Identifier: GPL-3.0-or-later
import type { ReactNode } from "react";

/**
 * The one icon set: a 1.6px stroke on a 24px grid with round caps and joins,
 * in the spirit of SF Symbols. Used everywhere instead of text glyphs
 * (x, <, >, check) or emoji, so every icon shares a weight and a rhythm.
 * Decorative by default - pair an icon-only button with an aria-label.
 */
export interface IconProps {
  size?: number;
  strokeWidth?: number;
  className?: string;
}

function icon(paths: ReactNode) {
  return function Icon({ size = 18, strokeWidth = 1.6, className }: IconProps) {
    return (
      <svg
        viewBox="0 0 24 24"
        width={size}
        height={size}
        fill="none"
        stroke="currentColor"
        strokeWidth={strokeWidth}
        strokeLinecap="round"
        strokeLinejoin="round"
        aria-hidden="true"
        className={className}
      >
        {paths}
      </svg>
    );
  };
}

export const IconClose = icon(<path d="M6 6l12 12M18 6 6 18" />);
export const IconChevronLeft = icon(<path d="m14.5 5.5-6.5 6.5 6.5 6.5" />);
export const IconChevronRight = icon(<path d="m9.5 5.5 6.5 6.5-6.5 6.5" />);
export const IconChevronUp = icon(<path d="m5.5 14.5 6.5-6.5 6.5 6.5" />);
export const IconChevronDown = icon(<path d="m5.5 9.5 6.5 6.5 6.5-6.5" />);
export const IconCheck = icon(<path d="m5 12.5 4.5 4.5L19 7.5" />);

export const IconCheckCircle = icon(
  <>
    <circle cx="12" cy="12" r="9" />
    <path d="m8 12.3 2.8 2.8L16 9.5" />
  </>,
);

export const IconWarning = icon(
  <>
    <path d="M12 3.5 21 19.5H3Z" />
    <path d="M12 10v4.2M12 16.8v.4" />
  </>,
);

export const IconInfo = icon(
  <>
    <circle cx="12" cy="12" r="9" />
    <path d="M12 11v5M12 7.8v.4" />
  </>,
);

export const IconDownload = icon(
  <>
    <path d="M12 4v11" />
    <path d="m7.5 11 4.5 4.5 4.5-4.5" />
    <path d="M5 20h14" />
  </>,
);

export const IconExpand = icon(
  <>
    <path d="M14 4h6v6" />
    <path d="M10 20H4v-6" />
    <path d="m20 4-7 7" />
    <path d="m4 20 7-7" />
  </>,
);

export const IconShare = icon(
  <>
    <path d="M12 15V4" />
    <path d="m8 7.5 4-4 4 4" />
    <path d="M5 12v6.5A1.5 1.5 0 0 0 6.5 20h11a1.5 1.5 0 0 0 1.5-1.5V12" />
  </>,
);

export const IconFolder = icon(
  <path d="M3.5 7.5a2 2 0 0 1 2-2h4l2 2.2h7a2 2 0 0 1 2 2v7.3a2 2 0 0 1-2 2h-13a2 2 0 0 1-2-2Z" />,
);

export const IconPhone = icon(
  <>
    <rect x="7" y="2.8" width="10" height="18.4" rx="2.4" />
    <path d="M10.6 18.3h2.8" />
  </>,
);

export const IconComputer = icon(
  <>
    <rect x="3" y="4" width="18" height="12" rx="2" />
    <path d="M8 20h8M12 16v4" />
  </>,
);

export const IconSearch = icon(
  <>
    <circle cx="11" cy="11" r="6.5" />
    <path d="m16 16 4.5 4.5" />
  </>,
);

export const IconTrash = icon(
  <>
    <path d="M4 7h16" />
    <path d="M9 7V4.5h6V7" />
    <path d="m6.5 7 .8 12a1.5 1.5 0 0 0 1.5 1.4h6.4a1.5 1.5 0 0 0 1.5-1.4L17.5 7" />
    <path d="M10 11v6M14 11v6" />
  </>,
);

export const IconFile = icon(
  <>
    <path d="M7 3.5h6.5L18 8v11.5a1 1 0 0 1-1 1H7a1 1 0 0 1-1-1v-15a1 1 0 0 1 1-1Z" />
    <path d="M13.5 3.5V8H18" />
  </>,
);

// Window controls (Windows custom title bar). Thinner, on a 10px grid feel.
export const IconWinMinimize = icon(<path d="M5 12h14" />);
export const IconWinMaximize = icon(<rect x="5.5" y="5.5" width="13" height="13" rx="1.5" />);
export const IconWinRestore = icon(
  <>
    <rect x="4.5" y="8.5" width="11" height="11" rx="1.5" />
    <path d="M8.5 8.5v-3a1.5 1.5 0 0 1 1.5-1.5h8a1.5 1.5 0 0 1 1.5 1.5v8a1.5 1.5 0 0 1-1.5 1.5h-2.5" />
  </>,
);

/** The Amber drop. Flat two-tone so it stays crisp from 16px to 64px. */
export function AppMark({ size = 24, className }: { size?: number; className?: string }) {
  return (
    <svg viewBox="0 0 24 24" width={size} height={size} aria-hidden="true" className={className}>
      <path
        d="M12 2.5c3.6 4.2 6.2 7.4 6.2 10.6A6.2 6.2 0 0 1 12 19.4a6.2 6.2 0 0 1-6.2-6.3C5.8 9.9 8.4 6.7 12 2.5Z"
        fill="#f2a73b"
      />
      <path
        d="M12 7.2c1.9 2.3 3.2 4 3.2 5.7a3.2 3.2 0 0 1-6.4 0c0-1.7 1.3-3.4 3.2-5.7Z"
        fill="#c97a12"
      />
    </svg>
  );
}
