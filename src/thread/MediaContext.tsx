// SPDX-License-Identifier: GPL-3.0-or-later
import { createContext, useContext } from "react";

export interface MediaActions {
  /** Open the full-screen viewer on one photo/video. `origin` is the thumbnail it grows out of. */
  open: (key: string, origin: Element | null) => void;
  /** Save an attachment to disk via a save dialog. */
  save: (relPath: string, filename: string) => void;
  /**
   * True while the timeline is being dragged. Media shows a lightweight
   * placeholder of the same size then, so fast scrubbing never starts
   * decoding hundreds of photos.
   */
  scrubbing: boolean;
}

const noop: MediaActions = { open: () => {}, save: () => {}, scrubbing: false };

export const MediaContext = createContext<MediaActions>(noop);

export function useMedia(): MediaActions {
  return useContext(MediaContext);
}
