// SPDX-License-Identifier: GPL-3.0-or-later
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open } from "@tauri-apps/plugin-dialog";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { ConversationView } from "./ConversationView";
import { ExportPanel } from "./export/ExportPanel";
import { Home } from "./home/Home";
import { ImportPanel } from "./import/ImportPanel";
import { importSummary } from "./import/importSummary";
import { getDayIndex, openArchive, queryMessages } from "./lib/api";
import { hasModifier } from "./lib/platform";
import {
  addRecent,
  loadRecents,
  removeRecent,
  saveRecents,
  type RecentArchive,
} from "./lib/recents";
import type { DayBucketDto, MessageDto, OpenArchiveResult } from "./lib/types";
import { useWindowChrome } from "./shell/chrome";
import { Toolbar } from "./shell/Toolbar";
import { LoadingThread } from "./thread/LoadingThread";
import { ContextMenuProvider } from "./ui/ContextMenu";
import { IconClose, IconInfo, IconWarning } from "./ui/icons";
import "./styles/index.css";

type LoadState =
  | { status: "home" }
  | { status: "loading" }
  | {
      status: "ready";
      conversation: OpenArchiveResult;
      messages: MessageDto[];
      dayBuckets: DayBucketDto[];
    };

interface Notice {
  kind: "info" | "error";
  text: string;
}

function conversationName(conversation: OpenArchiveResult): string {
  return conversation.display_name ?? conversation.chat_identifier;
}

function isArchivePath(path: string): boolean {
  return path.toLowerCase().endsWith(".amber");
}

function App() {
  useWindowChrome();

  const [state, setState] = useState<LoadState>({ status: "home" });
  const [importOpen, setImportOpen] = useState(false);
  const [exportOpen, setExportOpen] = useState(false);
  const [findOpen, setFindOpen] = useState(false);
  const [notice, setNotice] = useState<Notice | null>(null);
  const [recents, setRecents] = useState<RecentArchive[]>(loadRecents);
  const [dropping, setDropping] = useState(false);
  const opening = useRef(false);

  const ready = state.status === "ready";
  const sheetOpen = importOpen || exportOpen;

  const remember = useCallback((path: string, conversation: OpenArchiveResult) => {
    setRecents((list) => {
      const next = addRecent(list, {
        path,
        name: conversationName(conversation),
        messageCount: conversation.message_count,
        openedAt: Date.now(),
      });
      saveRecents(next);
      return next;
    });
  }, []);

  const loadCurrentArchive = useCallback(async (conversation: OpenArchiveResult) => {
    const [messages, dayBuckets] = await Promise.all([queryMessages(), getDayIndex()]);
    setFindOpen(false);
    setState({ status: "ready", conversation, messages, dayBuckets });
  }, []);

  const openPath = useCallback(
    async (path: string) => {
      if (opening.current) return;
      opening.current = true;
      setNotice(null);
      setState({ status: "loading" });
      try {
        const conversation = await openArchive(path);
        await loadCurrentArchive(conversation);
        remember(path, conversation);
      } catch (err) {
        setState({ status: "home" });
        setNotice({ kind: "error", text: `Amber couldn’t open that archive. ${String(err)}` });
      } finally {
        opening.current = false;
      }
    },
    [loadCurrentArchive, remember],
  );

  const handleOpen = useCallback(async () => {
    if (opening.current) return;
    const path = await open({
      multiple: false,
      filters: [{ name: "Amber Archive", extensions: ["amber"] }],
    });
    if (!path || Array.isArray(path)) return;
    await openPath(path);
  }, [openPath]);

  const handleImported = useCallback(
    (conversation: OpenArchiveResult, archivePath: string) => {
      setImportOpen(false);
      const summary = importSummary(conversation);
      setNotice(summary ? { kind: "info", text: summary } : null);
      setState({ status: "loading" });
      loadCurrentArchive(conversation)
        .then(() => remember(archivePath, conversation))
        .catch((err) => {
          setState({ status: "home" });
          setNotice({ kind: "error", text: `Amber couldn’t open the new archive. ${String(err)}` });
        });
    },
    [loadCurrentArchive, remember],
  );

  const goHome = useCallback(() => {
    setFindOpen(false);
    setState({ status: "home" });
  }, []);

  const forgetRecent = useCallback((path: string) => {
    setRecents((list) => {
      const next = removeRecent(list, path);
      saveRecents(next);
      return next;
    });
  }, []);

  const toggleFind = useCallback(() => setFindOpen((value) => !value), []);

  // Actions shared by the toolbar, the keyboard and the macOS menu bar.
  const actions = useMemo(
    () => ({
      open: () => void handleOpen(),
      import: () => setImportOpen(true),
      export: () => {
        if (ready) setExportOpen(true);
      },
      find: () => {
        if (ready) setFindOpen(true);
      },
    }),
    [handleOpen, ready],
  );
  const actionsRef = useRef(actions);
  useEffect(() => {
    actionsRef.current = actions;
  });

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (!hasModifier(event) || event.altKey || event.shiftKey || sheetOpen) return;
      const key = event.key.toLowerCase();
      const action = { o: "open", i: "import", e: "export", f: "find" }[key] as keyof typeof actions | undefined;
      if (!action) return;
      event.preventDefault();
      actionsRef.current[action]();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [sheetOpen]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;
    listen<string>("menu://action", (event) => {
      const action = event.payload as keyof typeof actions;
      if (!sheetOpen) actionsRef.current[action]?.();
    })
      .then((fn) => (cancelled ? fn() : (unlisten = fn)))
      .catch(() => {});
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [sheetOpen]);

  // The browser's own right-click menu (Reload, Inspect...) has no place in
  // an app. Fields and selectable text keep the standard one for Copy/Paste.
  useEffect(() => {
    const onContextMenu = (event: MouseEvent) => {
      const target = event.target as Element | null;
      if (target?.closest("input, textarea, .selectable")) return;
      event.preventDefault();
    };
    window.addEventListener("contextmenu", onContextMenu);
    return () => window.removeEventListener("contextmenu", onContextMenu);
  }, []);

  // Dropping an .amber file anywhere on the window opens it.
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;
    try {
      getCurrentWebview()
        .onDragDropEvent((event) => {
          const payload = event.payload;
          if (payload.type === "enter" || payload.type === "over") {
            setDropping(true);
          } else if (payload.type === "leave") {
            setDropping(false);
          } else if (payload.type === "drop") {
            setDropping(false);
            const path = payload.paths.find(isArchivePath);
            if (path) void openPath(path);
            else setNotice({ kind: "error", text: "That isn’t an Amber archive. Drop a file ending in .amber." });
          }
        })
        .then((fn) => (cancelled ? fn() : (unlisten = fn)))
        .catch(() => {});
    } catch {
      // Not inside the desktop shell.
    }
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [openPath]);

  const conversation = state.status === "ready" ? state.conversation : null;

  return (
    <ContextMenuProvider>
    <div className="app">
      <Toolbar
        conversation={
          conversation
            ? {
                title: conversationName(conversation),
                subtitle: `${conversation.message_count.toLocaleString()} messages`,
              }
            : null
        }
        findActive={findOpen}
        onHome={goHome}
        onOpen={actions.open}
        onImport={actions.import}
        onExport={actions.export}
        onFind={toggleFind}
      />

      {notice && (
        <div className={`banner banner-${notice.kind}`} role={notice.kind === "error" ? "alert" : "status"}>
          {notice.kind === "error" ? <IconWarning size={18} /> : <IconInfo size={18} />}
          <span className="banner-text selectable">{notice.text}</span>
          <button type="button" className="btn btn-icon" aria-label="Dismiss" onClick={() => setNotice(null)}>
            <IconClose size={16} />
          </button>
        </div>
      )}

      <main className="app-body">
        {state.status === "home" && (
          <Home
            recents={recents}
            onImport={actions.import}
            onOpen={actions.open}
            onOpenRecent={(path) => void openPath(path)}
            onRemoveRecent={forgetRecent}
          />
        )}
        {state.status === "loading" && <LoadingThread />}
        {state.status === "ready" && (
          <ConversationView
            messages={state.messages}
            dayBuckets={state.dayBuckets}
            isGroup={state.conversation.is_group}
            participants={state.conversation.participants}
            findOpen={findOpen}
            onFindClose={() => setFindOpen(false)}
          />
        )}
      </main>

      {dropping && (
        <div className="drop-overlay" aria-hidden="true">
          <div className="drop-card">Drop to open</div>
        </div>
      )}

      {exportOpen && state.status === "ready" && (
        <ExportPanel title={conversationName(state.conversation)} onClose={() => setExportOpen(false)} />
      )}

      {importOpen && <ImportPanel onClose={() => setImportOpen(false)} onImported={handleImported} />}
    </div>
    </ContextMenuProvider>
  );
}

export default App;
