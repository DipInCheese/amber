// SPDX-License-Identifier: GPL-3.0-or-later
//! Locates the `idevice_id` / `idevicebackup2` sidecar binaries: bundled
//! next to the app (via Tauri's resource dir) once M6 wires up
//! `externalBin`, falling back to PATH for local development.

use std::path::PathBuf;

use amber_ingest::Tools;
use tauri::{AppHandle, Manager};

pub fn resolve_tools(app: &AppHandle) -> Tools {
    Tools {
        idevice_id: resolve_one(app, "idevice_id"),
        idevicebackup2: resolve_one(app, "idevicebackup2"),
    }
}

fn resolve_one(app: &AppHandle, name: &str) -> PathBuf {
    let exe_name = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    };

    if let Ok(resource_dir) = app.path().resource_dir() {
        let bundled = resource_dir.join("bin").join(&exe_name);
        if bundled.is_file() {
            return bundled;
        }
    }

    // No bundled sidecar (expected until M6): fall back to PATH, so a
    // developer with libimobiledevice installed locally can still use the
    // device flow, and `check_prerequisites` reports clear guidance
    // otherwise (see amber_ingest::prerequisites).
    PathBuf::from(exe_name)
}
