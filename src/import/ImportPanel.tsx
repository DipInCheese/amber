// SPDX-License-Identifier: GPL-3.0-or-later
import { open, save } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useCallback, useEffect, useMemo, useState } from "react";
import {
  beginIngest,
  checkIngestPrerequisites,
  clearDeviceBackupCache,
  deviceBackupCacheInfo,
  ingestConversation,
  listIngestDevices,
  onBackupProgress,
  onWriteProgress,
  type WriteProgress,
} from "../lib/api";
import type {
  ConversationSummaryDto,
  DeviceBackupCacheDto,
  OpenArchiveResult,
  PrerequisiteReportDto,
  SourceRequest,
} from "../lib/types";
import { filterConversations, formatBytes, formatLastActive } from "./conversationFilter";
import { describeWriteProgress } from "./writeProgress";

type SourceKind = "device" | "backup" | "live_mac";

type Phase =
  | { step: "choose" }
  | { step: "configure"; kind: SourceKind }
  | { step: "resolving"; progress: number | null }
  | { step: "pick-conversation"; conversations: ConversationSummaryDto[] }
  | { step: "writing"; progress: WriteProgress | null }
  | { step: "error"; message: string };

function PrerequisiteRow({
  label,
  ok,
  guidance,
  remediationUrl,
}: {
  label: string;
  ok: boolean;
  guidance: string | null;
  remediationUrl: string | null;
}) {
  return (
    <div className={`prereq-row ${ok ? "prereq-ok" : "prereq-missing"}`}>
      <span className="prereq-icon">{ok ? "✓" : "✕"}</span>
      <div>
        <div className="prereq-label">{label}</div>
        {!ok && guidance && <div className="prereq-guidance">{guidance}</div>}
        {!ok && remediationUrl && (
          <button type="button" className="prereq-fix" onClick={() => openUrl(remediationUrl)}>
            Fix it
          </button>
        )}
      </div>
    </div>
  );
}

function WritingProgress({ progress }: { progress: WriteProgress | null }) {
  const { label, fraction } = describeWriteProgress(progress);
  return (
    <div className="import-progress">
      <p>{label}</p>
      <progress value={fraction ?? undefined} max={1} />
      <p className="import-note">
        Large conversations take a while. You can leave this window open; Amber stays usable.
      </p>
    </div>
  );
}

export function ImportPanel({
  onClose,
  onImported,
}: {
  onClose: () => void;
  onImported: (result: OpenArchiveResult) => void;
}) {
  const [phase, setPhase] = useState<Phase>({ step: "choose" });
  const [prereqs, setPrereqs] = useState<PrerequisiteReportDto | null>(null);
  const [devices, setDevices] = useState<string[]>([]);
  const [password, setPassword] = useState("");
  const [backupPath, setBackupPath] = useState<string | null>(null);
  const [cache, setCache] = useState<DeviceBackupCacheDto | null>(null);
  const [search, setSearch] = useState("");

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    onWriteProgress((progress) => {
      setPhase((current) => (current.step === "writing" ? { step: "writing", progress } : current));
    }).then((fn) => {
      unlisten = fn;
    });
    return () => unlisten?.();
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    onBackupProgress((percent) => {
      setPhase((current) =>
        current.step === "resolving" ? { step: "resolving", progress: percent } : current,
      );
    }).then((fn) => {
      unlisten = fn;
    });
    return () => unlisten?.();
  }, []);

  const refreshDevicePrereqs = useCallback(async () => {
    const report = await checkIngestPrerequisites();
    setPrereqs(report);
    if (report.idevice_id.ok) {
      setDevices(await listIngestDevices().catch(() => []));
    }
  }, []);

  const chooseKind = useCallback(
    async (kind: SourceKind) => {
      setPhase({ step: "configure", kind });
      if (kind === "device") {
        deviceBackupCacheInfo().then(setCache).catch(() => setCache(null));
        await refreshDevicePrereqs();
      }
      if (kind === "backup") {
        const path = await open({ directory: true, multiple: false });
        if (typeof path === "string") setBackupPath(path);
        else setPhase({ step: "choose" });
      }
    },
    [refreshDevicePrereqs],
  );

  const startResolve = useCallback(async () => {
    if (phase.step !== "configure") return;

    let source: SourceRequest;
    if (phase.kind === "live_mac") {
      source = { type: "live_mac" };
    } else if (phase.kind === "backup") {
      if (!backupPath) return;
      source = { type: "backup", path: backupPath, password: password || null };
    } else {
      source = { type: "device", udid: null, password: password || null };
    }

    setPhase({ step: "resolving", progress: null });
    try {
      const conversations = await beginIngest(source);
      setSearch("");
      setPhase({ step: "pick-conversation", conversations });
    } catch (err) {
      setPhase({ step: "error", message: String(err) });
    }
  }, [phase, backupPath, password]);

  /** Pick from the phone backup Amber already holds - no phone needed. */
  const startResolveFromCache = useCallback(async () => {
    if (!cache) return;
    setPhase({ step: "resolving", progress: null });
    try {
      const conversations = await beginIngest({
        type: "backup",
        path: cache.path,
        password: password || null,
      });
      setSearch("");
      setPhase({ step: "pick-conversation", conversations });
    } catch (err) {
      setPhase({ step: "error", message: String(err) });
    }
  }, [cache, password]);

  const deleteCache = useCallback(async () => {
    try {
      await clearDeviceBackupCache();
      setCache(null);
    } catch (err) {
      setPhase({ step: "error", message: String(err) });
    }
  }, []);

  const pickConversation = useCallback(async (chatIdentifier: string) => {
    const outputPath = await save({
      filters: [{ name: "Amber Archive", extensions: ["amber"] }],
      defaultPath: `${chatIdentifier.replace(/[^\w+.-]/g, "_")}.amber`,
    });
    if (!outputPath) return;

    setPhase({ step: "writing", progress: null });
    try {
      const result = await ingestConversation(chatIdentifier, outputPath);
      onImported(result);
    } catch (err) {
      setPhase({ step: "error", message: String(err) });
    }
  }, [onImported]);

  const visibleConversations = useMemo(
    () => (phase.step === "pick-conversation" ? filterConversations(phase.conversations, search) : []),
    [phase, search],
  );

  return (
    <div className="import-overlay">
      <div className="import-panel">
        <div className="import-header">
          <h2>Import a conversation</h2>
          <button type="button" className="import-close" onClick={onClose}>
            ×
          </button>
        </div>

        {phase.step === "choose" && (
          <div className="import-source-choices">
            <button type="button" onClick={() => chooseKind("device")}>
              📱 iPhone over USB
            </button>
            <button type="button" onClick={() => chooseKind("backup")}>
              💾 Existing backup
            </button>
            <button type="button" onClick={() => chooseKind("live_mac")}>
              🖥️ This Mac's Messages
            </button>
          </div>
        )}

        {phase.step === "configure" && phase.kind === "device" && (
          <div className="import-configure">
            {prereqs && (
              <>
                <PrerequisiteRow
                  label="idevice_id"
                  ok={prereqs.idevice_id.ok}
                  guidance={prereqs.idevice_id.guidance}
                  remediationUrl={prereqs.idevice_id.remediation_url}
                />
                <PrerequisiteRow
                  label="idevicebackup2"
                  ok={prereqs.idevicebackup2.ok}
                  guidance={prereqs.idevicebackup2.guidance}
                  remediationUrl={prereqs.idevicebackup2.remediation_url}
                />
                <PrerequisiteRow
                  label="USB driver"
                  ok={prereqs.platform.ok}
                  guidance={prereqs.platform.guidance}
                  remediationUrl={prereqs.platform.remediation_url}
                />
              </>
            )}
            <button type="button" onClick={refreshDevicePrereqs}>
              Recheck
            </button>
            <p>{devices.length > 0 ? `${devices.length} device(s) found.` : "No device found - plug in and trust this computer."}</p>
            <input
              type="password"
              placeholder="Backup password (only if encrypted)"
              value={password}
              onChange={(e) => setPassword(e.currentTarget.value)}
            />
            {cache && (
              <div className="import-cache">
                <p>
                  Your phone's last backup ({formatBytes(cache.size_bytes)}) is saved on this
                  computer. Choose a conversation from it without connecting your phone.
                </p>
                <button type="button" onClick={startResolveFromCache}>
                  Choose from saved backup
                </button>
                <button type="button" onClick={deleteCache}>
                  Delete saved backup
                </button>
              </div>
            )}
            <button
              type="button"
              disabled={!prereqs?.idevice_id.ok || !prereqs.idevicebackup2.ok || !prereqs.platform.ok}
              onClick={startResolve}
            >
              {cache ? "Update from phone (only changes are copied)" : "Back up phone and choose a conversation"}
            </button>
          </div>
        )}

        {phase.step === "configure" && phase.kind === "backup" && (
          <div className="import-configure">
            <p>{backupPath ?? "No folder selected."}</p>
            <input
              type="password"
              placeholder="Backup password (only if encrypted)"
              value={password}
              onChange={(e) => setPassword(e.currentTarget.value)}
            />
            <button type="button" disabled={!backupPath} onClick={startResolve}>
              Import
            </button>
          </div>
        )}

        {phase.step === "configure" && phase.kind === "live_mac" && (
          <div className="import-configure">
            <button type="button" onClick={startResolve}>
              Import from this Mac
            </button>
          </div>
        )}

        {phase.step === "resolving" && (
          <div className="import-progress">
            <p>
              {phase.progress !== null
                ? `Backing up… ${phase.progress.toFixed(0)}%`
                : "Reading conversations…"}
            </p>
            {phase.progress !== null && (
              <p className="import-note">
                iOS can only back up the whole phone, not one conversation. This is saved on this
                computer, so next time only what changed is copied, and you can pick other
                conversations without connecting your phone.
              </p>
            )}
          </div>
        )}

        {phase.step === "pick-conversation" && (
          <div className="import-conversation-list">
            {phase.conversations.length === 0 && <p>No conversations found.</p>}
            {phase.conversations.length > 0 && (
              <>
                <input
                  type="search"
                  className="import-search"
                  placeholder="Search by name, number or email"
                  value={search}
                  onChange={(e) => setSearch(e.currentTarget.value)}
                  autoFocus
                />
                <p className="import-note">
                  {visibleConversations.length === phase.conversations.length
                    ? `${phase.conversations.length} conversations, most recent first.`
                    : `${visibleConversations.length} of ${phase.conversations.length} conversations.`}
                </p>
              </>
            )}
            {visibleConversations.map((c) => (
              <button
                type="button"
                key={c.chat_identifier}
                className="import-conversation-row"
                onClick={() => pickConversation(c.chat_identifier)}
              >
                <span>
                  {c.display_name ?? c.chat_identifier}
                  {c.is_group && <span className="import-conversation-tag">Group</span>}
                </span>
                <span className="import-conversation-count">
                  {c.message_count} messages · {formatLastActive(c.last_message_ts_unix_ms)}
                </span>
              </button>
            ))}
            {phase.conversations.length > 0 && visibleConversations.length === 0 && (
              <p>No conversations match "{search}".</p>
            )}
          </div>
        )}

        {phase.step === "writing" && <WritingProgress progress={phase.progress} />}

        {phase.step === "error" && (
          <div className="import-error">
            <p>{phase.message}</p>
            <button type="button" onClick={() => setPhase({ step: "choose" })}>
              Start over
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
