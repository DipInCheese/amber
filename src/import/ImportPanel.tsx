// SPDX-License-Identifier: GPL-3.0-or-later
import { open, save } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import {
  beginIngest,
  checkIngestPrerequisites,
  clearDeviceBackupCache,
  deviceBackupCacheInfo,
  ingestConversation,
  listIngestDevices,
  onBackupLog,
  onBackupProgress,
  onWriteProgress,
  type WriteProgress,
} from "../lib/api";
import { isMacOS } from "../lib/platform";
import type {
  ConversationSummaryDto,
  DeviceBackupCacheDto,
  OpenArchiveResult,
  PrerequisiteReportDto,
  PrerequisiteStatusDto,
  SourceRequest,
} from "../lib/types";
import { Avatar } from "../ui/Avatar";
import {
  IconCheckCircle,
  IconChevronRight,
  IconFolder,
  IconPhone,
  IconSearch,
  IconTrash,
  IconWarning,
  IconComputer,
} from "../ui/icons";
import { ProgressBar } from "../ui/ProgressBar";
import { Sheet, SheetClose } from "../ui/Sheet";
import { filterConversations, formatBytes, formatLastActive } from "./conversationFilter";
import { describeWriteProgress } from "./writeProgress";

type SourceKind = "device" | "backup" | "live_mac";

type Phase =
  | { step: "choose" }
  | { step: "configure"; kind: SourceKind }
  | { step: "resolving"; progress: number | null; fromDevice: boolean }
  | { step: "pick-conversation"; conversations: ConversationSummaryDto[] }
  | { step: "writing"; progress: WriteProgress | null }
  | { step: "error"; message: string };

const BACKUP_DIR_KEY = "amber:backup-dir";
const LOG_LIMIT = 500;

function readStoredBackupDir(): string | null {
  try {
    return localStorage.getItem(BACKUP_DIR_KEY);
  } catch {
    return null;
  }
}

function storeBackupDir(dir: string | null) {
  try {
    if (dir) localStorage.setItem(BACKUP_DIR_KEY, dir);
    else localStorage.removeItem(BACKUP_DIR_KEY);
  } catch {
    // Remembering the folder is a convenience only.
  }
}

/** One line of the "is everything ready?" list. */
function CheckRow({
  title,
  ok,
  okDetail,
  problem,
  fixUrl,
}: {
  title: string;
  ok: boolean;
  okDetail: string;
  problem: string | null;
  fixUrl: string | null;
}) {
  return (
    <div className="list-row">
      <span className={`tile ${ok ? "tile-success" : "tile-warning"}`}>
        {ok ? <IconCheckCircle /> : <IconWarning />}
      </span>
      <span className="list-row-text">
        <span className="list-row-title">{title}</span>
        <span className="list-row-detail">{ok ? okDetail : (problem ?? "Not ready yet.")}</span>
      </span>
      {!ok && fixUrl && (
        <button type="button" className="btn btn-sm" onClick={() => void openUrl(fixUrl)}>
          Fix
        </button>
      )}
    </div>
  );
}

function statusOk(...statuses: PrerequisiteStatusDto[]): boolean {
  return statuses.every((status) => status.ok);
}

function LogView({ lines }: { lines: string[] }) {
  const ref = useRef<HTMLPreElement>(null);
  useEffect(() => {
    const el = ref.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [lines]);
  return (
    <pre ref={ref} className="log-view selectable" aria-label="Details">
      {lines.length > 0 ? lines.join("\n") : "Waiting for the first message from the phone…"}
    </pre>
  );
}

function Busy({
  title,
  detail,
  fraction,
  children,
}: {
  title: string;
  detail?: string;
  fraction: number | null;
  children?: ReactNode;
}) {
  return (
    <div className="busy">
      <div className="busy-head">
        <div className="spinner" aria-hidden="true" />
        <div>
          <p className="busy-title">{title}</p>
          {detail && <p className="note">{detail}</p>}
        </div>
      </div>
      <ProgressBar fraction={fraction} label={title} />
      {children}
    </div>
  );
}

export function ImportPanel({
  onClose,
  onImported,
}: {
  onClose: () => void;
  onImported: (result: OpenArchiveResult, archivePath: string) => void;
}) {
  const [phase, setPhase] = useState<Phase>({ step: "choose" });
  const [prereqs, setPrereqs] = useState<PrerequisiteReportDto | null>(null);
  const [checking, setChecking] = useState(false);
  const [devices, setDevices] = useState<string[]>([]);
  const [password, setPassword] = useState("");
  const [backupPath, setBackupPath] = useState<string | null>(null);
  const [backupDir, setBackupDir] = useState<string | null>(readStoredBackupDir);
  const [cache, setCache] = useState<DeviceBackupCacheDto | null>(null);
  const [search, setSearch] = useState("");
  const [log, setLog] = useState<string[]>([]);
  const [showLog, setShowLog] = useState(false);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;
    onWriteProgress((progress) => {
      setPhase((current) => (current.step === "writing" ? { ...current, progress } : current));
    }).then((fn) => (cancelled ? fn() : (unlisten = fn)));
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;
    onBackupProgress((percent) => {
      setPhase((current) => (current.step === "resolving" ? { ...current, progress: percent } : current));
    }).then((fn) => (cancelled ? fn() : (unlisten = fn)));
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;
    onBackupLog((line) => {
      setLog((lines) => {
        const next = lines.length >= LOG_LIMIT ? lines.slice(lines.length - LOG_LIMIT + 1) : lines.slice();
        next.push(line);
        return next;
      });
    }).then((fn) => (cancelled ? fn() : (unlisten = fn)));
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  const refreshDevicePrereqs = useCallback(async () => {
    setChecking(true);
    try {
      const report = await checkIngestPrerequisites();
      setPrereqs(report);
      setDevices(report.idevice_id.ok ? await listIngestDevices().catch(() => []) : []);
    } finally {
      setChecking(false);
    }
  }, []);

  const refreshCache = useCallback((dir: string | null) => {
    deviceBackupCacheInfo(dir)
      .then(setCache)
      .catch(() => setCache(null));
  }, []);

  const chooseKind = useCallback(
    async (kind: SourceKind) => {
      if (kind === "backup") {
        const path = await open({ directory: true, multiple: false, title: "Choose the backup folder" });
        if (typeof path !== "string") return;
        setBackupPath(path);
        setPhase({ step: "configure", kind });
        return;
      }
      setPhase({ step: "configure", kind });
      if (kind === "device") {
        refreshCache(backupDir);
        await refreshDevicePrereqs();
      }
    },
    [backupDir, refreshCache, refreshDevicePrereqs],
  );

  const changeBackupDir = useCallback(
    async (dir: string | null) => {
      setBackupDir(dir);
      storeBackupDir(dir);
      refreshCache(dir);
    },
    [refreshCache],
  );

  const pickBackupDir = useCallback(async () => {
    const dir = await open({ directory: true, multiple: false, title: "Where should Amber keep the phone backup?" });
    if (typeof dir === "string") await changeBackupDir(dir);
  }, [changeBackupDir]);

  const runResolve = useCallback(async (source: SourceRequest, fromDevice: boolean) => {
    setLog([]);
    setShowLog(false);
    setPhase({ step: "resolving", progress: null, fromDevice });
    try {
      const conversations = await beginIngest(source);
      setSearch("");
      setPhase({ step: "pick-conversation", conversations });
    } catch (err) {
      setPhase({ step: "error", message: String(err) });
    }
  }, []);

  const startResolve = useCallback(() => {
    if (phase.step !== "configure") return;
    if (phase.kind === "live_mac") return void runResolve({ type: "live_mac" }, false);
    if (phase.kind === "backup") {
      if (!backupPath) return;
      return void runResolve({ type: "backup", path: backupPath, password: password || null }, false);
    }
    void runResolve({ type: "device", udid: null, password: password || null, backup_dir: backupDir }, true);
  }, [phase, backupPath, password, backupDir, runResolve]);

  /** Pick from the phone backup Amber already holds - no phone needed. */
  const startResolveFromCache = useCallback(() => {
    if (!cache) return;
    void runResolve({ type: "backup", path: cache.path, password: password || null }, false);
  }, [cache, password, runResolve]);

  const deleteCache = useCallback(async () => {
    try {
      await clearDeviceBackupCache(backupDir);
      setCache(null);
    } catch (err) {
      setPhase({ step: "error", message: String(err) });
    }
  }, [backupDir]);

  const pickConversation = useCallback(
    async (chatIdentifier: string) => {
      const outputPath = await save({
        filters: [{ name: "Amber Archive", extensions: ["amber"] }],
        defaultPath: `${chatIdentifier.replace(/[^\w+.-]/g, "_")}.amber`,
      });
      if (!outputPath) return;

      setPhase({ step: "writing", progress: null });
      try {
        const result = await ingestConversation(chatIdentifier, outputPath);
        onImported(result, outputPath);
      } catch (err) {
        setPhase({ step: "error", message: String(err) });
      }
    },
    [onImported],
  );

  const visibleConversations = useMemo(
    () => (phase.step === "pick-conversation" ? filterConversations(phase.conversations, search) : []),
    [phase, search],
  );

  const deviceReady = prereqs ? statusOk(prereqs.idevice_id, prereqs.idevicebackup2, prereqs.platform) : false;
  const hasDevice = devices.length > 0;
  const backToChoose = () => setPhase({ step: "choose" });

  // --- Sheet frame per step ------------------------------------------------

  let title = "Import a conversation";
  let subtitle: string | undefined = "Choose where your messages are.";
  let footer: ReactNode = <SheetClose />;
  let body: ReactNode = null;

  if (phase.step === "choose") {
    body = (
      <div className="list-group">
        <button type="button" className="list-row" onClick={() => void chooseKind("device")}>
          <span className="tile tile-accent">
            <IconPhone />
          </span>
          <span className="list-row-text">
            <span className="list-row-title">iPhone</span>
            <span className="list-row-detail">Connect it with a USB cable</span>
          </span>
          <span className="list-row-trailing">
            <IconChevronRight size={16} />
          </span>
        </button>
        <button type="button" className="list-row" onClick={() => void chooseKind("backup")}>
          <span className="tile">
            <IconFolder />
          </span>
          <span className="list-row-text">
            <span className="list-row-title">A backup on this computer</span>
            <span className="list-row-detail">From Finder, iTunes or Apple Devices</span>
          </span>
          <span className="list-row-trailing">
            <IconChevronRight size={16} />
          </span>
        </button>
        {isMacOS && (
          <button type="button" className="list-row" onClick={() => void chooseKind("live_mac")}>
            <span className="tile">
              <IconComputer />
            </span>
            <span className="list-row-text">
              <span className="list-row-title">Messages on this Mac</span>
              <span className="list-row-detail">Read your conversations straight from this Mac</span>
            </span>
            <span className="list-row-trailing">
              <IconChevronRight size={16} />
            </span>
          </button>
        )}
      </div>
    );
  }

  if (phase.step === "configure" && phase.kind === "device") {
    title = "Import from iPhone";
    subtitle = "Amber copies your phone's backup to this computer, then lets you pick one conversation.";
    footer = (
      <>
        <button type="button" className="btn" onClick={backToChoose}>
          Back
        </button>
        <button type="button" className="btn btn-primary" disabled={!deviceReady || !hasDevice} onClick={startResolve}>
          {cache ? "Update from iPhone" : "Back up iPhone"}
        </button>
      </>
    );
    body = (
      <div className="stack">
        <section>
          <div className="section-head">
            <h3>Before we start</h3>
            <button type="button" className="btn btn-plain btn-sm" disabled={checking} onClick={() => void refreshDevicePrereqs()}>
              {checking ? "Checking…" : "Check again"}
            </button>
          </div>
          {prereqs ? (
            <div className="list-group">
              <CheckRow
                title="Amber's phone tools"
                ok={statusOk(prereqs.idevice_id, prereqs.idevicebackup2)}
                okDetail="Installed with Amber"
                problem={prereqs.idevice_id.guidance ?? prereqs.idevicebackup2.guidance}
                fixUrl={prereqs.idevice_id.remediation_url ?? prereqs.idevicebackup2.remediation_url}
              />
              <CheckRow
                title={isMacOS ? "Access to your files" : "Apple USB driver"}
                ok={prereqs.platform.ok}
                okDetail={isMacOS ? "Full Disk Access is on" : "Installed"}
                problem={prereqs.platform.guidance}
                fixUrl={prereqs.platform.remediation_url}
              />
              <CheckRow
                title="iPhone connected"
                ok={hasDevice}
                okDetail={devices.length === 1 ? "Ready" : `${devices.length} iPhones found`}
                problem="Plug in your iPhone, unlock it and tap Trust."
                fixUrl={null}
              />
            </div>
          ) : (
            <div className="list-group" aria-busy="true">
              <div className="list-row">
                <span className="tile skeleton" />
                <span className="list-row-text">
                  <span className="skeleton skeleton-line" style={{ width: "40%" }} />
                  <span className="skeleton skeleton-line" style={{ width: "65%" }} />
                </span>
              </div>
            </div>
          )}
        </section>

        {cache && (
          <section>
            <h3>Saved backup</h3>
            <div className="list-group">
              <div className="list-row">
                <span className="tile tile-success">
                  <IconCheckCircle />
                </span>
                <span className="list-row-text">
                  <span className="list-row-title">{formatBytes(cache.size_bytes)} on this computer</span>
                  <span className="list-row-detail">Choose a conversation from it without your phone.</span>
                </span>
                <button type="button" className="btn btn-sm" onClick={startResolveFromCache}>
                  Choose
                </button>
                <button type="button" className="btn btn-icon" aria-label="Delete saved backup" onClick={() => void deleteCache()}>
                  <IconTrash />
                </button>
              </div>
            </div>
          </section>
        )}

        <section>
          <label className="field-label" htmlFor="backup-password">
            Backup password
          </label>
          <input
            id="backup-password"
            className="field"
            type="password"
            placeholder="Only if you encrypted your backup"
            value={password}
            onChange={(e) => setPassword(e.currentTarget.value)}
          />
        </section>

        <section>
          <span className="field-label">Keep the backup in</span>
          <div className="list-group">
            <div className="list-row">
              <span className="tile">
                <IconFolder />
              </span>
              <span className="list-row-text">
                <span className="list-row-title">{backupDir ?? "Amber's own folder"}</span>
                <span className="list-row-detail">
                  Needs about as much free space as your phone uses. Next time, only changes are copied.
                </span>
              </span>
              <button type="button" className="btn btn-sm" onClick={() => void pickBackupDir()}>
                Change…
              </button>
              {backupDir && (
                <button type="button" className="btn btn-plain btn-sm" onClick={() => void changeBackupDir(null)}>
                  Reset
                </button>
              )}
            </div>
          </div>
        </section>
      </div>
    );
  }

  if (phase.step === "configure" && phase.kind === "backup") {
    title = "Import from a backup";
    subtitle = undefined;
    footer = (
      <>
        <button type="button" className="btn" onClick={backToChoose}>
          Back
        </button>
        <button type="button" className="btn btn-primary" disabled={!backupPath} onClick={startResolve}>
          Continue
        </button>
      </>
    );
    body = (
      <div className="stack">
        <section>
          <span className="field-label">Backup folder</span>
          <div className="list-group">
            <div className="list-row">
              <span className="tile">
                <IconFolder />
              </span>
              <span className="list-row-text">
                <span className="list-row-title selectable">{backupPath ?? "No folder chosen"}</span>
              </span>
              <button type="button" className="btn btn-sm" onClick={() => void chooseKind("backup")}>
                Change…
              </button>
            </div>
          </div>
        </section>
        <section>
          <label className="field-label" htmlFor="backup-password">
            Backup password
          </label>
          <input
            id="backup-password"
            className="field"
            type="password"
            placeholder="Only if the backup is encrypted"
            value={password}
            onChange={(e) => setPassword(e.currentTarget.value)}
          />
        </section>
      </div>
    );
  }

  if (phase.step === "configure" && phase.kind === "live_mac") {
    title = "Import from this Mac";
    subtitle = "Amber reads your Messages database without changing it.";
    footer = (
      <>
        <button type="button" className="btn" onClick={backToChoose}>
          Back
        </button>
        <button type="button" className="btn btn-primary" onClick={startResolve}>
          Continue
        </button>
      </>
    );
    body = (
      <p className="note">
        macOS asks you to allow Full Disk Access for Amber the first time. Open System Settings → Privacy &amp;
        Security → Full Disk Access if it doesn’t.
      </p>
    );
  }

  if (phase.step === "resolving") {
    title = phase.fromDevice ? "Backing up your iPhone" : "Reading conversations";
    subtitle = undefined;
    footer = (
      <>
        <button type="button" className="btn btn-plain" onClick={() => setShowLog((v) => !v)}>
          {showLog ? "Hide details" : "Show details"}
        </button>
        <SheetClose>Close</SheetClose>
      </>
    );
    body = (
      <Busy
        title={
          phase.progress !== null
            ? `Backing up… ${phase.progress.toFixed(0)}%`
            : phase.fromDevice
              ? "Starting the backup…"
              : "Reading conversations…"
        }
        detail={
          phase.fromDevice
            ? "iOS can only back up the whole phone, not one conversation. The backup stays on this computer, so next time only what changed is copied."
            : undefined
        }
        fraction={phase.progress !== null ? phase.progress / 100 : null}
      >
        {showLog && <LogView lines={log} />}
      </Busy>
    );
  }

  if (phase.step === "pick-conversation") {
    title = "Choose a conversation";
    subtitle = undefined;
    footer = (
      <>
        <button type="button" className="btn" onClick={backToChoose}>
          Start over
        </button>
        <SheetClose />
      </>
    );
    body = (
      <div className="stack">
        {phase.conversations.length === 0 ? (
          <p className="note">No conversations were found in this backup.</p>
        ) : (
          <>
            <label className="search-field">
              <IconSearch size={16} />
              <input
                type="search"
                autoFocus
                placeholder="Search by name, number or email"
                value={search}
                onChange={(e) => setSearch(e.currentTarget.value)}
              />
            </label>
            <p className="note">
              {visibleConversations.length === phase.conversations.length
                ? `${phase.conversations.length.toLocaleString()} conversations, most recent first.`
                : `${visibleConversations.length.toLocaleString()} of ${phase.conversations.length.toLocaleString()} conversations.`}
            </p>
            <div className="list-group conversation-list">
              {visibleConversations.map((c) => {
                const name = c.display_name ?? c.chat_identifier;
                return (
                  <button
                    type="button"
                    key={c.chat_identifier}
                    className="list-row"
                    onClick={() => void pickConversation(c.chat_identifier)}
                  >
                    <Avatar name={name} size={32} />
                    <span className="list-row-text">
                      <span className="list-row-title">
                        {name}
                        {c.is_group && <span className="tag">Group</span>}
                      </span>
                      <span className="list-row-detail">
                        {c.message_count.toLocaleString()} messages · {formatLastActive(c.last_message_ts_unix_ms)}
                      </span>
                    </span>
                    <span className="list-row-trailing">
                      <IconChevronRight size={16} />
                    </span>
                  </button>
                );
              })}
              {visibleConversations.length === 0 && (
                <p className="note empty-note">No conversations match “{search}”.</p>
              )}
            </div>
          </>
        )}
      </div>
    );
  }

  if (phase.step === "writing") {
    const view = describeWriteProgress(phase.progress);
    title = "Saving your conversation";
    subtitle = undefined;
    footer = <SheetClose>Close</SheetClose>;
    body = (
      <Busy
        title={view.label}
        detail="Large conversations take a while. Amber stays usable."
        fraction={view.fraction}
      />
    );
  }

  if (phase.step === "error") {
    title = "That didn’t work";
    subtitle = undefined;
    footer = (
      <>
        <SheetClose>Close</SheetClose>
        <button type="button" className="btn btn-primary" onClick={backToChoose}>
          Try again
        </button>
      </>
    );
    body = (
      <div className="stack">
        <div className="list-group">
          <div className="list-row">
            <span className="tile tile-danger">
              <IconWarning />
            </span>
            <span className="list-row-text">
              <span className="list-row-title">Amber couldn’t finish the import</span>
              <span className="list-row-detail selectable">{phase.message}</span>
            </span>
          </div>
        </div>
        {log.length > 0 && (
          <>
            <button type="button" className="btn btn-plain btn-sm align-start" onClick={() => setShowLog((v) => !v)}>
              {showLog ? "Hide details" : "Show details"}
            </button>
            {showLog && <LogView lines={log} />}
          </>
        )}
      </div>
    );
  }

  return (
    <Sheet title={title} subtitle={subtitle} footer={footer} onClose={onClose} width={phase.step === "pick-conversation" ? 520 : 480}>
      {body}
    </Sheet>
  );
}
