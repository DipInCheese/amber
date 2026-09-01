// SPDX-License-Identifier: GPL-3.0-or-later
use std::path::PathBuf;

/// Errors that can occur while writing or reading a `.amber` archive.
#[derive(Debug, thiserror::Error)]
pub enum AmberFormatError {
    #[error("message {0:?} references unknown sender participant {1:?}")]
    UnknownSender(String, String),

    #[error("attachment on message {0:?} references unknown participant {1:?}")]
    UnknownReactionParticipant(String, String),

    #[error("attachment source file not found: {0}")]
    AttachmentSourceMissing(PathBuf),

    #[error("not a ZIP archive: {0}")]
    NotAZip(String),

    #[error("archive is missing {0}")]
    MissingEntry(&'static str),

    #[error("manifest.json is not valid JSON: {0}")]
    InvalidManifest(#[from] serde_json::Error),

    #[error("not an Amber archive: manifest 'format' field is {0:?}, expected \"amber-archive\"")]
    WrongFormat(String),

    #[error(
        "messages.db has application_id {found:#010x}, expected {expected:#010x} ('AMBR') - not an Amber database"
    )]
    WrongApplicationId { found: i64, expected: i64 },

    #[error("archive format_version {found} has a newer major version than this reader supports ({supported_major})")]
    UnsupportedFormatVersion { found: String, supported_major: u64 },

    #[error("archive schema_version {found} is newer than this reader supports ({supported})")]
    UnsupportedSchemaVersion { found: i64, supported: i64 },

    #[error(
        "messages.db integrity check failed: manifest says sha256 {expected}, actual is {actual}"
    )]
    IntegrityMismatch { expected: String, actual: String },

    #[error("invalid semver in format_version {0:?}: {1}")]
    InvalidSemver(String, String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("ZIP error: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
}
