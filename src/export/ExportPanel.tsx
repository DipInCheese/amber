// SPDX-License-Identifier: GPL-3.0-or-later
import { save } from "@tauri-apps/plugin-dialog";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { useEffect, useState, type ReactNode } from "react";
import {
  exportConversation,
  onExportProgress,
  type ExportFormat,
  type ExportProgress,
  type ExportSummary,
} from "../lib/api";
import { safeFileName } from "../lib/media";
import { IconCheck, IconCheckCircle, IconFile, IconWarning } from "../ui/icons";
import { ProgressBar } from "../ui/ProgressBar";
import { Sheet, SheetClose } from "../ui/Sheet";
import { describeExport, EXPORT_FORMATS } from "./exportSummary";

type Phase =
  | { step: "choose" }
  | { step: "exporting"; progress: ExportProgress | null }
  | { step: "done"; summary: ExportSummary }
  | { step: "error"; message: string };

/** Export the open conversation to a web page, text, spreadsheet or data file. */
export function ExportPanel({ title, onClose }: { title: string; onClose: () => void }) {
  const [format, setFormat] = useState<ExportFormat>("html");
  const [includeMedia, setIncludeMedia] = useState(true);
  const [phase, setPhase] = useState<Phase>({ step: "choose" });

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;
    onExportProgress((progress) => {
      setPhase((current) => (current.step === "exporting" ? { step: "exporting", progress } : current));
    }).then((fn) => (cancelled ? fn() : (unlisten = fn)));
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  const info = EXPORT_FORMATS.find((f) => f.id === format) ?? EXPORT_FORMATS[0];

  const start = async () => {
    const outputPath = await save({
      defaultPath: `${safeFileName(title)}.${info.extension}`,
      filters: [{ name: info.label, extensions: [info.extension] }],
    });
    if (!outputPath) return;

    setPhase({ step: "exporting", progress: null });
    try {
      const summary = await exportConversation(format, format === "html" && includeMedia, outputPath);
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
    footer = <SheetClose>Close</SheetClose>;
    const fraction =
      phase.progress && phase.progress.total > 0 ? phase.progress.done / phase.progress.total : null;
    const label =
      phase.progress && phase.progress.total > 0
        ? `Copying media… ${phase.progress.done.toLocaleString()} of ${phase.progress.total.toLocaleString()}`
        : "Writing the file…";
    body = (
      <div className="busy">
        <div className="busy-head">
          <div className="spinner" aria-hidden="true" />
          <div>
            <p className="busy-title">{label}</p>
            <p className="note">Amber stays usable while this runs.</p>
          </div>
        </div>
        <ProgressBar fraction={fraction} label={label} />
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
