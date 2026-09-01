// SPDX-License-Identifier: GPL-3.0-or-later
//! Opens and validates a `.amber` archive, per the order in `SPEC.md` §4:
//! it is a ZIP -> `manifest.json` parses and `format == "amber-archive"` ->
//! `application_id == 0x414D4252` -> versions are in range -> (optional)
//! `messages_db_sha256` matches.

use std::fs::File;
use std::io::Read as _;
use std::path::{Path, PathBuf};

use rusqlite::Connection;
use zip::ZipArchive;

use crate::error::AmberFormatError;
use crate::hash::sha256_hex;
use crate::manifest::Manifest;
use crate::model::{
    AttachmentRecord, DayBucketRecord, MessageRecord, ParticipantRecord, ReactionRecord,
};
use crate::schema::{APPLICATION_ID, SCHEMA_VERSION};

/// A validated, opened `.amber` archive.
///
/// `messages.db` is extracted to a temp file for the lifetime of this
/// value (SQLite needs a real file, not bytes inside a ZIP); attachment
/// bytes are read from the ZIP on demand via [`AmberArchive::read_attachment`].
pub struct AmberArchive {
    manifest: Manifest,
    zip_path: PathBuf,
    db_temp_path: tempfile::TempPath,
}

impl AmberArchive {
    /// Open and fully validate the archive at `path`.
    pub fn open(path: &Path) -> Result<Self, AmberFormatError> {
        let file = File::open(path)?;
        let mut zip =
            ZipArchive::new(file).map_err(|err| AmberFormatError::NotAZip(err.to_string()))?;

        let manifest = read_manifest(&mut zip)?;
        manifest.validate_format()?;

        let db_temp_path = extract_messages_db(&mut zip)?;
        verify_integrity(&db_temp_path, &manifest)?;
        verify_application_id(&db_temp_path)?;
        verify_schema_version(&db_temp_path, &manifest)?;

        Ok(Self {
            manifest,
            zip_path: path.to_path_buf(),
            db_temp_path,
        })
    }

    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    fn db(&self) -> Result<Connection, AmberFormatError> {
        Ok(Connection::open_with_flags(
            &self.db_temp_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )?)
    }

    pub fn list_participants(&self) -> Result<Vec<ParticipantRecord>, AmberFormatError> {
        let db = self.db()?;
        let mut stmt =
            db.prepare("SELECT id, identifier, display_name, is_me FROM participant ORDER BY id")?;
        let rows = stmt.query_map([], |row| {
            Ok(ParticipantRecord {
                id: row.get(0)?,
                identifier: row.get(1)?,
                display_name: row.get(2)?,
                is_me: row.get(3)?,
            })
        })?;
        rows.collect::<Result<_, _>>().map_err(Into::into)
    }

    /// All messages in the archive, ordered chronologically.
    pub fn list_messages(&self) -> Result<Vec<MessageRecord>, AmberFormatError> {
        let db = self.db()?;
        let mut stmt = db.prepare(
            "SELECT id, guid, participant_id, is_from_me, ts_unix_ms, service, text,
                    reply_to_guid, is_edited, is_unsent, edit_history
             FROM message ORDER BY ts_unix_ms",
        )?;
        let rows = stmt.query_map([], |row| {
            let edit_history_json: Option<String> = row.get(10)?;
            Ok((
                MessageRecord {
                    id: row.get(0)?,
                    guid: row.get(1)?,
                    participant_id: row.get(2)?,
                    is_from_me: row.get(3)?,
                    ts_unix_ms: row.get(4)?,
                    service: row.get(5)?,
                    text: row.get(6)?,
                    reply_to_guid: row.get(7)?,
                    is_edited: row.get(8)?,
                    is_unsent: row.get(9)?,
                    edit_history: None,
                },
                edit_history_json,
            ))
        })?;

        let mut messages = Vec::new();
        for row in rows {
            let (mut message, edit_history_json) = row?;
            message.edit_history = edit_history_json
                .map(|json| serde_json::from_str(&json))
                .transpose()?;
            messages.push(message);
        }
        Ok(messages)
    }

    pub fn list_attachments(&self) -> Result<Vec<AttachmentRecord>, AmberFormatError> {
        let db = self.db()?;
        let mut stmt = db.prepare(
            "SELECT id, message_id, sha256, rel_path, mime_type, filename, byte_size,
                    width, height, duration_ms, thumb_path
             FROM attachment ORDER BY id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(AttachmentRecord {
                id: row.get(0)?,
                message_id: row.get(1)?,
                sha256: row.get(2)?,
                rel_path: row.get(3)?,
                mime_type: row.get(4)?,
                filename: row.get(5)?,
                byte_size: row.get(6)?,
                width: row.get(7)?,
                height: row.get(8)?,
                duration_ms: row.get(9)?,
                thumb_path: row.get(10)?,
            })
        })?;
        rows.collect::<Result<_, _>>().map_err(Into::into)
    }

    pub fn list_reactions(&self) -> Result<Vec<ReactionRecord>, AmberFormatError> {
        let db = self.db()?;
        let mut stmt = db.prepare(
            "SELECT id, target_message_guid, participant_id, kind, emoji, ts_unix_ms, is_removed
             FROM reaction ORDER BY id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ReactionRecord {
                id: row.get(0)?,
                target_message_guid: row.get(1)?,
                participant_id: row.get(2)?,
                kind: row.get(3)?,
                emoji: row.get(4)?,
                ts_unix_ms: row.get(5)?,
                is_removed: row.get(6)?,
            })
        })?;
        rows.collect::<Result<_, _>>().map_err(Into::into)
    }

    /// The `day_index` scrubber cache, ordered by day.
    pub fn list_day_buckets(&self) -> Result<Vec<DayBucketRecord>, AmberFormatError> {
        let db = self.db()?;
        let mut stmt = db.prepare(
            "SELECT day, message_count, first_msg_id, last_msg_id FROM day_index ORDER BY day",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(DayBucketRecord {
                day: row.get(0)?,
                message_count: row.get(1)?,
                first_msg_id: row.get(2)?,
                last_msg_id: row.get(3)?,
            })
        })?;
        rows.collect::<Result<_, _>>().map_err(Into::into)
    }

    /// Read one attachment's bytes out of the archive by its `rel_path`
    /// (e.g. `attachments/ab/abcdef....heic`, as stored in the `attachment`
    /// table).
    pub fn read_attachment(&self, rel_path: &str) -> Result<Vec<u8>, AmberFormatError> {
        let file = File::open(&self.zip_path)?;
        let mut zip =
            ZipArchive::new(file).map_err(|err| AmberFormatError::NotAZip(err.to_string()))?;
        let mut entry = zip.by_name(rel_path)?;
        let mut bytes = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut bytes)?;
        Ok(bytes)
    }
}

fn read_manifest(zip: &mut ZipArchive<File>) -> Result<Manifest, AmberFormatError> {
    let mut entry = zip
        .by_name("manifest.json")
        .map_err(|_| AmberFormatError::MissingEntry("manifest.json"))?;
    let mut bytes = Vec::with_capacity(entry.size() as usize);
    entry.read_to_end(&mut bytes)?;
    Ok(serde_json::from_slice(&bytes)?)
}

fn extract_messages_db(zip: &mut ZipArchive<File>) -> Result<tempfile::TempPath, AmberFormatError> {
    let mut entry = zip
        .by_name("messages.db")
        .map_err(|_| AmberFormatError::MissingEntry("messages.db"))?;
    let mut bytes = Vec::with_capacity(entry.size() as usize);
    entry.read_to_end(&mut bytes)?;
    drop(entry);

    let temp_file = tempfile::NamedTempFile::new()?;
    std::fs::write(&temp_file, &bytes)?;
    Ok(temp_file.into_temp_path())
}

fn verify_integrity(db_path: &Path, manifest: &Manifest) -> Result<(), AmberFormatError> {
    let bytes = std::fs::read(db_path)?;
    let actual = sha256_hex(&bytes);
    if actual != manifest.integrity.messages_db_sha256 {
        return Err(AmberFormatError::IntegrityMismatch {
            expected: manifest.integrity.messages_db_sha256.clone(),
            actual,
        });
    }
    Ok(())
}

fn verify_application_id(db_path: &Path) -> Result<(), AmberFormatError> {
    let db = Connection::open_with_flags(db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let found: i64 = db.query_row("PRAGMA application_id", [], |row| row.get(0))?;
    if found != APPLICATION_ID {
        return Err(AmberFormatError::WrongApplicationId {
            found,
            expected: APPLICATION_ID,
        });
    }
    Ok(())
}

fn verify_schema_version(db_path: &Path, manifest: &Manifest) -> Result<(), AmberFormatError> {
    if manifest.schema_version > SCHEMA_VERSION {
        return Err(AmberFormatError::UnsupportedSchemaVersion {
            found: manifest.schema_version,
            supported: SCHEMA_VERSION,
        });
    }

    let db = Connection::open_with_flags(db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let found: i64 = db.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if found > SCHEMA_VERSION {
        return Err(AmberFormatError::UnsupportedSchemaVersion {
            found,
            supported: SCHEMA_VERSION,
        });
    }

    Ok(())
}
