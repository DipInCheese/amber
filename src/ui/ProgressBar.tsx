// SPDX-License-Identifier: GPL-3.0-or-later

/** A slim progress bar. `fraction` 0-1, or `null` for an indeterminate sweep. */
export function ProgressBar({ fraction, label }: { fraction: number | null; label: string }) {
  const determinate = fraction !== null;
  return (
    <div
      className={`progress${determinate ? "" : " progress-indeterminate"}`}
      role="progressbar"
      aria-label={label}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={determinate ? Math.round(Math.min(1, Math.max(0, fraction)) * 100) : undefined}
    >
      <div
        className="progress-fill"
        style={determinate ? { width: `${Math.min(1, Math.max(0, fraction)) * 100}%` } : undefined}
      />
    </div>
  );
}
