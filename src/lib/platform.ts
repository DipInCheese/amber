// SPDX-License-Identifier: GPL-3.0-or-later

const agent = typeof navigator === "undefined" ? "" : navigator.userAgent;

export const isMacOS = /Macintosh|Mac OS X/i.test(agent);
export const isWindows = /Windows/i.test(agent);

/** The shortcut modifier as the user knows it. */
export const modKeyLabel = isMacOS ? "⌘" : "Ctrl";

export function hasModifier(event: { metaKey: boolean; ctrlKey: boolean }): boolean {
  return isMacOS ? event.metaKey : event.ctrlKey;
}
