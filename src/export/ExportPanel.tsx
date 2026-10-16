// SPDX-License-Identifier: GPL-3.0-or-later
import { save } from "@tauri-apps/plugin-dialog";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { useEffect, useState, type ReactNode } from "react";
import {
  exportConversation,
  onExportLog,
  onExportProgress,
  type ExportFormat,
  type ExportProgress,
  type ExportSummary,
} from "../lib/api";
import { safeFileName } from "../lib/media";
import { IconCheck, IconCheckCircle, IconFile, IconWarning } from "../ui/icons";
import { appendLog, LogView } from "../ui/LogView";
import { ProgressBar } from "../ui/ProgressBar";
import { Sheet, SheetClose } from "../ui/Sheet";
import { describeExport, describeExportProgress, EXPORT_FORMATS } from "./exportSummary";

type Phase =
  | { step: "choose" }
  | { step: "exporting"; progress: ExportProgress | null; withMedia: boolean; startedAt: number }
  | { step: "done"; summary: ExportSummary }
  | { step: "error"; message: string };

/** Export the open conversation to a web page, text, spreadsheet or data file. */
export function ExportPanel({ title, onClose }: { title: string; onClose: () => void }) {
  const [format, setFormat] = useState<ExportFormat>("html");
  const [includeMedia, setIncludeMedia] = useState(true);
  const [phase, setPhase] = useState<Phase>({ step: "choose" });
  const [log, setLog] = useState<string[]>([]);
  const [showLog, setShowLog] = useState(true);
  const [now, setNow] = useState(() => Date.now());

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;
    onExportProgress((progress) => {
      setPhase((current) => (current.step === "exporting" ? { ...current, progress } : current));
    }).then((fn) => (cancelled ? fn() : (unlisten = fn)));
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;
    onExportLog((line) => setLog((lines) => appendLog(lines, line))).then((fn) =>
      cancelled ? fn() : (unlisten = fn),
    );
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  // A running clock, so a long export visibly isn't stuck.
  const exporting = phase.step === "exporting";
  useEffect(() => {
    if (!exporting) return;
    const timer = window.setInterval(() => setNow(Date.now()), 500);
    return () => window.clearInterval(timer);
  }, [exporting]);

  const info = EXPORT_FORMATS.find((f) => f.id === format) ?? EXPORT_FORMATS[0];

  const start = async () => {
    const outputPath = await save({
      defaultPath: `${safeFileName(title)}.${info.extension}`,
      filters: [{ name: info.label, extensions: [info.extension] }],
    });
    if (!outputPath) return;

    const withMedia = format === "html" && includeMedia;
    setLog([]);
    setNow(Date.now());
    setPhase({ step: "exporting", progress: null, withMedia, startedAt: Date.now() });
    try {
      const summary = await exportConversation(format, withMedia, outputPath);
      setPhase({ step: "done", summary });
    } catch (err) {
      setPhase({ step: "error", message: String(err) });
    }
  };

  let sheetTitle = "Export conversation";
  let footer: ReactNode = <SheetClose />;
  let body: ReactNode = null;

  if (phase.step === "choose") {
    footer = (
      <>
        <SheetClose />
        <button type="button" className="btn btn-primary" onClick={() => void start()}>
          Export…
        </button>
      </>
    );
    body = (
      <div className="stack">
        <section>
          <span className="field-label" id="export-format-label">
            Format
          </span>
          <div className="list-group" role="radiogroup" aria-labelledby="export-format-label">
            {EXPORT_FORMATS.map((option) => {
              const selected = format === option.id;
              return (
                <button
                  type="button"
                  key={option.id}
                  role="radio"
                  aria-checked={selected}
                  className="list-row"
                  onClick={() => setFormat(option.id)}
                >
                  <span className={`tile ${selected ? "tile-accent" : ""}`}>
                    <IconFile />
                  </span>
                  <span className="list-row-text">
                    <span className="list-row-title">{option.label}</span>
                    <span className="list-row-detail">{option.description}</span>
                  </span>
                  <span className={`radio-mark${selected ? " is-on" : ""}`}>{selected && <IconCheck size={16} strokeWidth={2.2} />}</span>
                </button>
              );
            })}
          </div>
        </section>

        {format === "html" && (
          <section>
            <label className="switch-row">
              <span className="list-row-text">
                <span className="list-row-title">Include photos and videos</span>
                <span className="list-row-detail">
                  Copied into a folder next to the page. A conversation with lots of media can take many gigabytes.
                </span>
              </span>
              <input
                type="checkbox"
                className="switch"
                role="switch"
                checked={includeMedia}
                onChange={(e) => setIncludeMedia(e.currentTarget.checked)}
              />
            </label>
          </section>
        )}
      </div>
    );
  }

  if (phase.step === "exporting") {
    sheetTitle = "Exporting…";
    footer = (
      <>
        <button type="button" className="btn btn-plain" onClick={() => setShowLog((v) => !v)}>
          {showLog ? "Hide details" : "Show details"}
        </button>
        <SheetClose>Close</SheetClose>
      </>
    );
    const view = describeExportProgress(phase.progress, phase.withMedia);
    const elapsed = Math.max(0, Math.round((now - phase.startedAt) / 1000));
    const percent = view.fraction === null ? null : Math.floor(view.fraction * 100);
    body = (
      <div className="busy">
        <div className="busy-head">
          <div className="spinner" aria-hidden="true" />
          <div className="busy-text">
            <p className="busy-title">{view.label}</p>
            <p className="note">
              {view.step}
              {percent !== null && ` · ${percent}%`} · {elapsed}s · Amber stays usable while this runs.
            </p>
          </div>
        </div>
        <ProgressBar fraction={view.fraction} label={view.label} />
        {showLog && <LogView lines={log} />}
      </div>
    );
  }

  if (phase.step === "done") {
    sheetTitle = "Export finished";
    footer = (
      <>
        <button type="button" className="btn" onClick={() => void revealItemInDir(phase.summary.path)}>
          Show in folder
        </button>
        <SheetClose primary>Done</SheetClose>
      </>
    );
    body = (
      <div className="stack">
        <div className="list-group">
          <div className="list-row">
            <span className="tile tile-success">
              <IconCheckCircle />
            </span>
            <span className="list-row-text">
              <span className="list-row-title">Your conversation is ready</span>
              <span className="list-row-detail">{describeExport(phase.summary)}</span>
            </span>
          </div>
        </div>
        {log.length > 0 && <LogView lines={log} />}
      </div>
    );
  }

  if (phase.step === "error") {
    sheetTitle = "That didn’t work";
    footer = (
      <>
        <SheetClose>Close</SheetClose>
        <button type="button" className="btn btn-primary" onClick={() => setPhase({ step: "choose" })}>
          Back
        </button>
      </>
    );
    body = (
      <div className="list-group">
        <div className="list-row">
          <span className="tile tile-danger">
            <IconWarning />
          </span>
          <span className="list-row-text">
            <span className="list-row-title">Amber couldn’t export this conversation</span>
            <span className="list-row-detail selectable">{phase.message}</span>
          </span>
        </div>
      </div>
    );
  }

  return (
    <Sheet title={sheetTitle} subtitle={phase.step === "choose" ? title : undefined} footer={footer} onClose={onClose}>
      {body}
    </Sheet>
  );
}
