// SPDX-License-Identifier: GPL-3.0-or-later
export interface ToastMessage {
  id: number;
  text: string;
  kind: "info" | "error";
}

/** A brief, non-blocking confirmation ("Saved IMG_0001.HEIC") that eases in and out. */
export function Toast({ toast }: { toast: ToastMessage | null }) {
  return (
    <div className="toast-region" role="status" aria-live="polite">
      {toast && (
        <div key={toast.id} className={`toast toast-${toast.kind}`}>
          {toast.text}
        </div>
      )}
    </div>
  );
}
