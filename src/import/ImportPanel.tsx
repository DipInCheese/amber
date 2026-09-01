// SPDX-License-Identifier: GPL-3.0-or-later
import { open, save } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useCallback, useEffect, useState } from "react";
import {
  beginIngest,
  checkIngestPrerequisites,
  ingestConversation,
  listIngestDevices,
  onBackupProgress,
} from "../lib/api";
import type {
  ConversationSummaryDto,
  OpenArchiveResult,
  PrerequisiteReportDto,
  SourceRequest,
} from "../lib/types";

type SourceKind = "device" | "backup" | "live_mac";

type Phase =
  | { step: "choose" }
  | { step: "configure"; kind: SourceKind }
  | { step: "resolving"; progress: number | null }
  | { step: "pick-conversation"; conversations: ConversationSummaryDto[] }
  | { step: "writing" }
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
      if (kind === "device") await refreshDevicePrereqs();
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
      setPhase({ step: "pick-conversation", conversations });
    } catch (err) {
      setPhase({ step: "error", message: String(err) });
    }
  }, [phase, backupPath, password]);

  const pickConversation = useCallback(async (chatIdentifier: string) => {
    const outputPath = await save({
      filters: [{ name: "Amber Archive", extensions: ["amber"] }],
      defaultPath: `${chatIdentifier.replace(/[^\w+.-]/g, "_")}.amber`,
    });
    if (!outputPath) return;

    setPhase({ step: "writing" });
    try {
      const result = await ingestConversation(chatIdentifier, outputPath);
      onImported(result);
    } catch (err) {
      setPhase({ step: "error", message: String(err) });
    }
  }, [onImported]);

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
            <button
              type="button"
              disabled={!prereqs?.idevice_id.ok || !prereqs.idevicebackup2.ok || !prereqs.platform.ok}
              onClick={startResolve}
            >
              Back up and import
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
          </div>
        )}

        {phase.step === "pick-conversation" && (
          <div className="import-conversation-list">
            {phase.conversations.length === 0 && <p>No conversations found.</p>}
            {phase.conversations.map((c) => (
              <button
                type="button"
                key={c.chat_identifier}
                className="import-conversation-row"
                onClick={() => pickConversation(c.chat_identifier)}
              >
                <span>{c.display_name ?? c.chat_identifier}</span>
                <span className="import-conversation-count">{c.message_count} messages</span>
              </button>
            ))}
          </div>
        )}

        {phase.step === "writing" && <p>Writing .amber archive…</p>}

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
