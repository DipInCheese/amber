// SPDX-License-Identifier: GPL-3.0-or-later
use std::path::PathBuf;

use crate::error::IngestError;

/// Where a conversation's Messages database comes from. All three resolve
/// to "a `chat.db`-shaped SQLite file amber-core can parse, plus a way to
/// fetch attachment bytes by the path imessage-database reports for them."
#[derive(Debug, Clone)]
pub enum Source {
    /// An iPhone connected over USB. Amber creates or refreshes a local
    /// backup via bundled `idevicebackup2`, then treats it like `Backup`.
    Device {
        /// A specific device's UDID, or `None` to use the only connected
        /// device (an error if there's more than one).
        udid: Option<String>,
    },
    /// An existing iTunes / Apple Devices backup already on disk.
    Backup {
        /// Path to the backup's UDID folder (containing `Manifest.plist`).
        path: PathBuf,
        /// Required only if the backup is encrypted.
        password: Option<String>,
    },
    /// `~/Library/Messages/chat.db` on the local machine (macOS only;
    /// requires Full Disk Access).
    LiveMac,
}

/// A [`Source`] resolved down to a readable Messages database and a way to
/// fetch the bytes behind any attachment path amber-core reports.
pub struct ResolvedSource {
    /// Path to a `chat.db`-shaped SQLite file, in Amber's own working
    /// directory - never the original device/backup/live file.
    pub chat_db_path: PathBuf,
    /// Human-readable device name, when known (from backup/device
    /// metadata; `None` for a live Mac).
    pub device_name: Option<String>,
    attachments: Box<dyn AttachmentLocator>,
}

impl ResolvedSource {
    #[must_use]
    pub fn new(
        chat_db_path: PathBuf,
        device_name: Option<String>,
        attachments: Box<dyn AttachmentLocator>,
    ) -> Self {
        Self {
            chat_db_path,
            device_name,
            attachments,
        }
    }

    /// Resolve one attachment's bytes, given the path
    /// `imessage_database::tables::attachment::Attachment::filename()` (or
    /// `.path()`) reported for it. Source-specific: see the `AttachmentLocator`
    /// impl for each [`Source`] variant.
    pub fn attachment_bytes(&self, path: &str) -> Result<Vec<u8>, IngestError> {
        self.attachments.resolve(path)
    }
}

/// Resolves the on-device attachment path reported by `imessage-database`
/// to actual bytes. What that requires differs entirely by source: a live
/// Mac's paths are real, directly-readable filesystem paths; a backup's
/// paths have to be matched against `Manifest.db` and possibly decrypted.
///
/// `Send` only (not `Sync`): a backup-backed locator wraps a
/// `rusqlite::Connection`, which isn't `Sync`. A `ResolvedSource` is used
/// from one Tauri command invocation at a time, never shared across threads
/// concurrently, so `Send` (movable into that command's task) is enough.
pub trait AttachmentLocator: Send {
    fn resolve(&self, path: &str) -> Result<Vec<u8>, IngestError>;
}
