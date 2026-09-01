// SPDX-License-Identifier: GPL-3.0-or-later
import { formatDateSeparator } from "./formatting";

export function DateSeparator({ tsUnixMs }: { tsUnixMs: number }) {
  return (
    <div className="date-separator">
      <span>{formatDateSeparator(tsUnixMs)}</span>
    </div>
  );
}
