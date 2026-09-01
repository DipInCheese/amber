// SPDX-License-Identifier: GPL-3.0-or-later
import { open } from "@tauri-apps/plugin-dialog";
import { useCallback, useState } from "react";
import "./App.css";
import { ConversationView } from "./ConversationView";
import { getDayIndex, openArchive, queryMessages } from "./lib/api";
import type { DayBucketDto, MessageDto, OpenArchiveResult } from "./lib/types";

type LoadState =
  | { status: "empty" }
  | { status: "loading" }
  | { status: "error"; message: string }
  | {
      status: "ready";
      conversation: OpenArchiveResult;
      messages: MessageDto[];
      dayBuckets: DayBucketDto[];
    };

function App() {
  const [state, setState] = useState<LoadState>({ status: "empty" });

  const handleOpen = useCallback(async () => {
    const path = await open({
      multiple: false,
      filters: [{ name: "Amber Archive", extensions: ["amber"] }],
    });
    if (!path || Array.isArray(path)) return;

    setState({ status: "loading" });
    try {
      const conversation = await openArchive(path);
      const [messages, dayBuckets] = await Promise.all([queryMessages(), getDayIndex()]);
      setState({ status: "ready", conversation, messages, dayBuckets });
    } catch (err) {
      setState({ status: "error", message: String(err) });
    }
  }, []);

  return (
    <div className="app">
      <header className="app-header">
        <button type="button" className="open-button" onClick={handleOpen}>
          Open .amber…
        </button>
        {state.status === "ready" && (
          <div className="conversation-title">
            {state.conversation.display_name ?? state.conversation.chat_identifier}
            <span className="conversation-count">
              {state.conversation.message_count.toLocaleString()} messages
            </span>
          </div>
        )}
      </header>

      <main className="app-body">
        {state.status === "empty" && (
          <div className="placeholder">Open a .amber archive to view the conversation.</div>
        )}
        {state.status === "loading" && <div className="placeholder">Loading…</div>}
        {state.status === "error" && (
          <div className="placeholder placeholder-error">Couldn't open archive: {state.message}</div>
        )}
        {state.status === "ready" && (
          <ConversationView
            messages={state.messages}
            dayBuckets={state.dayBuckets}
            isGroup={state.conversation.is_group}
          />
        )}
      </main>
    </div>
  );
}

export default App;
