// SPDX-License-Identifier: GPL-3.0-or-later
import { isWindows, modKeyLabel } from "../lib/platform";
import { IconChevronLeft, IconFolder, IconSearch, IconShare } from "../ui/icons";
import { WindowControls } from "./WindowControls";

export interface ToolbarProps {
  /** `null` on the home screen. */
  conversation: { title: string; subtitle: string } | null;
  findActive: boolean;
  onHome: () => void;
  onOpen: () => void;
  onImport: () => void;
  onExport: () => void;
  onFind: () => void;
}

/**
 * The window's title bar. Dragging it moves the window; on Windows it also
 * carries our own window buttons. One filled button (Import) leads, the rest
 * are quiet.
 */
export function Toolbar({ conversation, findActive, onHome, onOpen, onImport, onExport, onFind }: ToolbarProps) {
  return (
    <header className="toolbar" data-tauri-drag-region>
      {conversation ? (
        <>
          <button type="button" className="btn btn-plain btn-sm toolbar-back" onClick={onHome}>
            <IconChevronLeft size={16} />
            Archives
          </button>
          <div className="toolbar-title" data-tauri-drag-region>
            <span className="toolbar-name">{conversation.title}</span>
            <span className="toolbar-sub">{conversation.subtitle}</span>
          </div>
        </>
      ) : (
        <div className="toolbar-title" data-tauri-drag-region>
          <span className="toolbar-name">Amber</span>
        </div>
      )}

      <div className="toolbar-spacer" data-tauri-drag-region />

      {conversation && (
        <div className="toolbar-actions">
          <button
            type="button"
            className="btn btn-icon"
            aria-label="Find in conversation"
            aria-pressed={findActive}
            title={`Find (${modKeyLabel}+F)`}
            onClick={onFind}
          >
            <IconSearch />
          </button>
          <button
            type="button"
            className="btn btn-icon"
            aria-label="Export conversation"
            title={`Export (${modKeyLabel}+E)`}
            onClick={onExport}
          >
            <IconShare />
          </button>
          <span className="toolbar-divider" aria-hidden="true" />
          <button type="button" className="btn btn-plain btn-sm" title={`Open (${modKeyLabel}+O)`} onClick={onOpen}>
            <IconFolder size={16} />
            Open
          </button>
          <button type="button" className="btn btn-primary btn-sm" title={`Import (${modKeyLabel}+I)`} onClick={onImport}>
            Import
          </button>
        </div>
      )}

      {isWindows && <WindowControls />}
    </header>
  );
}
