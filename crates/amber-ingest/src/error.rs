// SPDX-License-Identifier: GPL-3.0-or-later
use std::path::PathBuf;

/// Errors that can occur while resolving a [`crate::Source`] to a Messages
/// database + attachments.
#[derive(Debug, thiserror::Error)]
pub enum IngestError {
    #[error("could not find the Messages database in this backup")]
    MessagesDatabaseNotFound,

    #[error("could not find attachment {0:?} in this backup")]
    AttachmentNotFoundInBackup(String),

    #[error(
        "no live Messages database at {0}: on macOS this usually means Amber needs Full Disk Access"
    )]
    LiveMacDatabaseNotFound(PathBuf),

    #[error("{0} is not on PATH or in the bundled sidecar directory: {1}")]
    ToolNotFound(&'static str, String),

    #[error("{tool} exited with status {status}: {stderr}")]
    ToolFailed {
        tool: &'static str,
        status: String,
        stderr: String,
    },

    #[error("no iOS device is connected (or it hasn't been trusted yet)")]
    NoDeviceFound,

    #[error("multiple iOS devices are connected ({0:?}); specify which one to use")]
    AmbiguousDevice(Vec<String>),

    #[error("iOS backup error: {0}")]
    Backup(#[from] crabapple::error::BackupError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
