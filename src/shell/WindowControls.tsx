// SPDX-License-Identifier: GPL-3.0-or-later
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useState } from "react";
import { IconClose, IconWinMaximize, IconWinMinimize, IconWinRestore } from "../ui/icons";

/** Minimise / maximise / close for the Windows title bar we draw ourselves. */
export function WindowControls() {
  const [maximized, setMaximized] = useState(false);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;
    try {
      const win = getCurrentWindow();
      const sync = () => void win.isMaximized().then(setMaximized).catch(() => {});
      sync();
      win
        .onResized(sync)
        .then((fn) => (cancelled ? fn() : (unlisten = fn)))
        .catch(() => {});
    } catch {
      // Not inside the desktop shell.
    }
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  const run = (action: "minimize" | "toggleMaximize" | "close") => () => {
    try {
      void getCurrentWindow()[action]();
    } catch {
      // Not inside the desktop shell.
    }
  };

  return (
    <div className="win-controls">
      <button type="button" aria-label="Minimize" onClick={run("minimize")}>
        <IconWinMinimize size={16} />
      </button>
      <button type="button" aria-label={maximized ? "Restore" : "Maximize"} onClick={run("toggleMaximize")}>
        {maximized ? <IconWinRestore size={16} /> : <IconWinMaximize size={16} />}
      </button>
      <button type="button" className="win-close" aria-label="Close" onClick={run("close")}>
        <IconClose size={16} />
      </button>
    </div>
  );
}
