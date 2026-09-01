// SPDX-License-Identifier: GPL-3.0-or-later
//! Resolves one of the three [`Source`] variants (device over USB, an
//! existing iOS/iTunes backup, or a live Mac's `chat.db`) down to "a
//! Messages SQLite database + a way to fetch attachment bytes," which
//! `amber-core::parse` can then read.
//!
//! `Source::Backup` and `Source::LiveMac` resolve with just a path (see
//! [`resolve_backup`] / [`resolve_live_mac`]). `Source::Device` additionally
//! needs a [`command_runner::CommandRunner`] and [`device::Tools`] (where
//! the bundled sidecars live is a Tauri-layer concern - see
//! [`resolve_device`]), and reports progress as it creates/refreshes the
//! backup, so it's exposed separately rather than through one signature
//! that would have to accommodate all three.

mod backup;
mod command_runner;
mod device;
mod error;
mod live_mac;
mod prerequisites;
mod source;

pub use command_runner::{CommandOutput, CommandRunner, SystemCommandRunner};
pub use device::{list_devices, select_device, Tools};
pub use error::IngestError;
pub use prerequisites::{
    check_prerequisites, install_usb_driver, PrerequisiteReport, PrerequisiteStatus,
};
pub use source::{AttachmentLocator, ResolvedSource, Source};

use std::path::Path;

/// Extract (decrypting if needed) the Messages database from an existing
/// iTunes / Apple Devices backup into `work_dir`.
pub fn resolve_backup(
    path: &Path,
    password: Option<String>,
    work_dir: &Path,
) -> Result<ResolvedSource, IngestError> {
    backup::resolve(path, password, work_dir)
}

/// Copy the local machine's live Messages database into `work_dir`
/// (macOS only; requires Full Disk Access).
pub fn resolve_live_mac(work_dir: &Path) -> Result<ResolvedSource, IngestError> {
    live_mac::resolve(work_dir)
}

/// Create or refresh a backup of the device identified by `udid` (or the
/// sole connected device, if `None`) into `backups_root`, reporting 0-100
/// progress via `on_progress`, then resolve it exactly like
/// [`resolve_backup`].
#[allow(clippy::too_many_arguments)]
pub fn resolve_device(
    runner: &dyn CommandRunner,
    tools: &Tools,
    udid: Option<&str>,
    password: Option<&str>,
    backups_root: &Path,
    work_dir: &Path,
    on_progress: impl FnMut(f32),
) -> Result<ResolvedSource, IngestError> {
    let udid = device::select_device(runner, tools, udid)?;
    let backup_dir = device::create_or_refresh_backup(
        runner,
        tools,
        &udid,
        backups_root,
        password,
        on_progress,
    )?;
    backup::resolve(&backup_dir, password.map(str::to_string), work_dir)
}
