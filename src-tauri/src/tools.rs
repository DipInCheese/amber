// SPDX-License-Identifier: GPL-3.0-or-later
//! Identifies the `idevice_id` / `idevicebackup2` tools by name.
//!
//! Resolving *where* they actually live at runtime - a bundled Tauri
//! sidecar, or a plain `PATH` lookup for local development - is
//! `sidecar::SidecarCommandRunner`'s job, not this module's; `Tools` here
//! just carries the two names through `amber_ingest`'s API.

use amber_ingest::Tools;

pub fn tools() -> Tools {
    Tools {
        idevice_id: "idevice_id".into(),
        idevicebackup2: "idevicebackup2".into(),
    }
}
