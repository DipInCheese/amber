// SPDX-License-Identifier: GPL-3.0-or-later
//! Reads `~/Library/Messages/chat.db` directly (macOS only; requires Full
//! Disk Access - see `prerequisites.rs`).
//!
//! **Read-only at the source, always**: the live database is only ever
//! copied into Amber's own working directory, never opened in place.

use std::path::{Path, PathBuf};

use crate::error::IngestError;
use crate::source::{AttachmentLocator, ResolvedSource};

/// Copy the local machine's live Messages database into `work_dir` and
/// return a [`ResolvedSource`] for it.
pub fn resolve(work_dir: &Path) -> Result<ResolvedSource, IngestError> {
    let home =
        dirs::home_dir().ok_or_else(|| IngestError::LiveMacDatabaseNotFound(PathBuf::from("~")))?;
    resolve_from_home(&home, work_dir)
}

/// The testable core of [`resolve`]: takes the home directory explicitly
/// rather than reading it from the environment, so tests can point it at a
/// controlled temp directory instead of the real machine's home - which, on
/// an actual Mac (including GitHub's `macos-latest` runners), already has a
/// `~/Library/Messages/chat.db` even on a fresh image.
fn resolve_from_home(home: &Path, work_dir: &Path) -> Result<ResolvedSource, IngestError> {
    let live_db_path = home.join("Library/Messages/chat.db");

    if !live_db_path.is_file() {
        return Err(IngestError::LiveMacDatabaseNotFound(live_db_path));
    }

    let chat_db_path = work_dir.join("chat.db");
    std::fs::copy(&live_db_path, &chat_db_path)?;

    // Best-effort: WAL/SHM sidecars carry rows SQLite hasn't checkpointed
    // into chat.db yet. Copying them (when present) alongside chat.db lets
    // the reader pick them up; their absence just means slightly-stale data,
    // not a hard failure.
    for suffix in ["-wal", "-shm"] {
        let mut source = live_db_path.clone().into_os_string();
        source.push(suffix);
        let source = PathBuf::from(source);
        if source.is_file() {
            let mut dest = chat_db_path.clone().into_os_string();
            dest.push(suffix);
            let _ = std::fs::copy(&source, PathBuf::from(dest));
        }
    }

    Ok(ResolvedSource::new(
        chat_db_path,
        None,
        Box::new(LiveMacAttachmentLocator {
            home: home.to_path_buf(),
        }),
    ))
}

struct LiveMacAttachmentLocator {
    home: PathBuf,
}

impl AttachmentLocator for LiveMacAttachmentLocator {
    fn resolve(&self, path: &str) -> Result<Vec<u8>, IngestError> {
        let expanded = match path.strip_prefix("~/") {
            Some(rest) => self.home.join(rest),
            None => PathBuf::from(path),
        };
        Ok(std::fs::read(&expanded)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attachment_locator_expands_home_tilde() {
        let temp = tempfile::tempdir().unwrap();
        let attachments_dir = temp.path().join("Library/Messages/Attachments");
        std::fs::create_dir_all(&attachments_dir).unwrap();
        std::fs::write(attachments_dir.join("photo.heic"), b"fake image bytes").unwrap();

        let locator = LiveMacAttachmentLocator {
            home: temp.path().to_path_buf(),
        };

        let bytes = locator
            .resolve("~/Library/Messages/Attachments/photo.heic")
            .unwrap();
        assert_eq!(bytes, b"fake image bytes");
    }

    #[test]
    fn attachment_locator_passes_through_absolute_paths() {
        let temp = tempfile::tempdir().unwrap();
        let file_path = temp.path().join("photo.heic");
        std::fs::write(&file_path, b"fake image bytes").unwrap();

        let locator = LiveMacAttachmentLocator {
            home: PathBuf::from("/unused"),
        };

        let bytes = locator.resolve(file_path.to_str().unwrap()).unwrap();
        assert_eq!(bytes, b"fake image bytes");
    }

    #[test]
    fn missing_live_database_is_a_clear_error() {
        let fake_home = tempfile::tempdir().unwrap();
        let work_dir = tempfile::tempdir().unwrap();

        let result = resolve_from_home(fake_home.path(), work_dir.path());

        assert!(matches!(
            result,
            Err(IngestError::LiveMacDatabaseNotFound(_))
        ));
    }

    #[test]
    fn copies_the_live_database_and_wal_shm_sidecars_when_present() {
        let fake_home = tempfile::tempdir().unwrap();
        let work_dir = tempfile::tempdir().unwrap();

        let messages_dir = fake_home.path().join("Library/Messages");
        std::fs::create_dir_all(&messages_dir).unwrap();
        std::fs::write(messages_dir.join("chat.db"), b"fake chat.db bytes").unwrap();
        std::fs::write(messages_dir.join("chat.db-wal"), b"fake wal bytes").unwrap();
        // No -shm sidecar, to prove its absence doesn't fail the copy.

        let resolved = resolve_from_home(fake_home.path(), work_dir.path()).unwrap();

        assert_eq!(
            std::fs::read(&resolved.chat_db_path).unwrap(),
            b"fake chat.db bytes"
        );
        let wal_path = {
            let mut p = resolved.chat_db_path.clone().into_os_string();
            p.push("-wal");
            PathBuf::from(p)
        };
        assert_eq!(std::fs::read(&wal_path).unwrap(), b"fake wal bytes");
    }
}
