// SPDX-License-Identifier: GPL-3.0-or-later
import { save } from "@tauri-apps/plugin-dialog";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { useEffect, useState } from "react";
import {
  exportConversation,
  onExportProgress,
  type ExportFormat,
  type ExportProgress,
  type ExportSummary,
} from "../lib/api";
import { safeFileName } from "../lib/media";
import { describeExport, EXPORT_FORMATS } from "./exportSummary";

type Phase =
  | { step: "choose" }
  | { step: "exporting"; progress: ExportProgress | null }
  | { step: "done"; summary: ExportSummary }
  | { step: "error"; message: string };

/** Export the open conversation to a text, CSV, JSON or HTML file. */
export function ExportPanel({ title, onClose }: { title: string; onClose: () => void }) {
  const [format, setFormat] = useState<ExportFormat>("html");
  const [includeMedia, setIncludeMedia] = useState(true);
  const [phase, setPhase] = useState<Phase>({ step: "choose" });

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    onExportProgress((progress) => {
      setPhase((current) => (current.step === "exporting" ? { step: "exporting", progress } : current));
    }).then((fn) => {
      unlisten = fn;
    });
    return () => unlisten?.();
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

  return (
    <div className="import-overlay">
      <div className="import-panel" role="dialog" aria-modal="true" aria-label="Export conversation">
        <div className="import-header">
          <h2>Export conversation</h2>
          <button type="button" className="import-close" aria-label="Close" onClick={onClose}>
            ×
          </button>
        </div>

        {phase.step === "choose" && (
          <div className="import-configure">
            <div className="export-formats" role="radiogroup" aria-label="Format">
              {EXPORT_FORMATS.map((option) => (
                <label key={option.id} className={`export-format${format === option.id ? " is-selected" : ""}`}>
                  <input
                    type="radio"
                    name="export-format"
                    value={option.id}
                    checked={format === option.id}
                    onChange={() => setFormat(option.id)}
                  />
                  <span className="export-format-text">
                    <strong>{option.label}</strong>
                    <span>{option.description}</span>
                  </span>
                </label>
              ))}
            </div>

            {format === "html" && (
              <label className="export-media-toggle">
                <input
                  type="checkbox"
                  checked={includeMedia}
                  onChange={(e) => setIncludeMedia(e.currentTarget.checked)}
                />
                Include photos and videos
                <span className="import-note">
                  Copied into a folder next to the page. A conversation with lots of media can be
                  many gigabytes.
                </span>
              </label>
            )}

            <button type="button" onClick={start}>
              Export…
            </button>
          </div>
        )}

        {phase.step === "exporting" && (
          <div className="import-progress">
            <p>
              {phase.progress && phase.progress.total > 0
                ? `Copying media… ${phase.progress.done.toLocaleString()} of ${phase.progress.total.toLocaleString()}`
                : "Exporting…"}
            </p>
            <progress
              value={phase.progress && phase.progress.total > 0 ? phase.progress.done / phase.progress.total : undefined}
              max={1}
            />
            <p className="import-note">Amber stays usable while this runs.</p>
          </div>
        )}

        {phase.step === "done" && (
          <div className="import-progress">
            <p>{describeExport(phase.summary)}</p>
            <div className="export-done-actions">
              <button type="button" onClick={() => void revealItemInDir(phase.summary.path)}>
                Show in folder
              </button>
              <button type="button" onClick={onClose}>
                Done
              </button>
            </div>
          </div>
        )}

        {phase.step === "error" && (
          <div className="import-error">
            <p>Couldn't export: {phase.message}</p>
            <button type="button" onClick={() => setPhase({ step: "choose" })}>
              Back
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
