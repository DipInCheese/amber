// SPDX-License-Identifier: GPL-3.0-or-later
import { useEffect, useState } from "react";
import { checkIngestPrerequisites } from "./lib/api";
import type { PrerequisiteReportDto } from "./lib/types";

const SEEN_KEY = "amber:onboarding-seen";

/**
 * First-run prerequisite check (definition-of-done item 2: "checks
 * prerequisites and clearly guides me through any that are missing"). Runs
 * once per browser profile (`localStorage`, which persists across launches
 * in the WebView) - silent if everything's already in place, since there's
 * nothing to guide the user through in that case.
 */
export function OnboardingBanner() {
  const [report, setReport] = useState<PrerequisiteReportDto | null>(null);
  const [dismissed, setDismissed] = useState(false);

  useEffect(() => {
    if (localStorage.getItem(SEEN_KEY)) return;
    checkIngestPrerequisites()
      .then((result) => {
        if (!result.idevice_id.ok || !result.idevicebackup2.ok || !result.platform.ok) {
          setReport(result);
        } else {
          localStorage.setItem(SEEN_KEY, "1");
        }
      })
      .catch(() => {
        // First-run guidance is a nice-to-have; a failed check here
        // shouldn't block using the app at all.
      });
  }, []);

  const dismiss = () => {
    localStorage.setItem(SEEN_KEY, "1");
    setDismissed(true);
  };

  if (!report || dismissed) return null;

  const missing = [
    !report.idevice_id.ok && report.idevice_id.guidance,
    !report.idevicebackup2.ok && report.idevicebackup2.guidance,
    !report.platform.ok && report.platform.guidance,
  ].filter((g): g is string => Boolean(g));

  return (
    <div className="onboarding-banner">
      <div>
        <strong>Before you plug in an iPhone:</strong>
        <ul>
          {missing.map((guidance) => (
            <li key={guidance}>{guidance}</li>
          ))}
        </ul>
        <span className="onboarding-note">
          Opening an existing .amber file or importing from a backup file works either way.
        </span>
      </div>
      <button type="button" onClick={dismiss}>
        Got it
      </button>
    </div>
  );
}
