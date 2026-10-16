// SPDX-License-Identifier: GPL-3.0-or-later

const SHAPES: Array<{ me: boolean; width: number }> = [
  { me: false, width: 46 },
  { me: false, width: 62 },
  { me: true, width: 38 },
  { me: false, width: 54 },
  { me: true, width: 70 },
  { me: true, width: 42 },
  { me: false, width: 58 },
];

/** Shimmering stand-in bubbles while an archive loads, so the window never looks empty or frozen. */
export function LoadingThread() {
  return (
    <div className="loading-thread" role="status" aria-label="Loading messages">
      {SHAPES.map((shape, index) => (
        <div key={index} className={`loading-row ${shape.me ? "from-me" : "from-them"}`}>
          <span className="skeleton loading-bubble" style={{ width: `${shape.width}%` }} />
        </div>
      ))}
    </div>
  );
}
