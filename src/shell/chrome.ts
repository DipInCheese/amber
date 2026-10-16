// SPDX-License-Identifier: GPL-3.0-or-later
import { invoke } from "@tauri-apps/api/core";
import { useEffect } from "react";
import { isMacOS, isWindows } from "../lib/platform";

/**
 * Tells the stylesheet which window it is living in: `platform-windows` /
 * `platform-macos` for the custom title bar, and `mica` once Windows reports
 * the translucent system backdrop is active (the toolbar then stays
 * see-through instead of painting its own background).
 */
export function useWindowChrome(): void {
  useEffect(() => {
    const root = document.documentElement;
    if (isWindows) root.classList.add("platform-windows");
    if (isMacOS) root.classList.add("platform-macos");
    invoke<{ mica: boolean }>("chrome_info")
      .then((info) => root.classList.toggle("mica", info.mica))
      .catch(() => {
        // Not running inside the desktop shell (or an older one): opaque toolbar.
      });
  }, []);
}
