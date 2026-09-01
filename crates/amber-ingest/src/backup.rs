// SPDX-License-Identifier: GPL-3.0-or-later
//! Reads an iTunes / Apple Devices backup via `crabapple`. Handles both
//! unencrypted backups (the common case - Messages data isn't encrypted
//! unless the device owner turned on encrypted backups) and encrypted ones.

use std::collections::HashMap;
use std::path::Path;

use crabapple::backup::models::auth::Authentication;
use crabapple::backup::models::file::BackupFileEntry;
use crabapple::Backup;

use crate::error::IngestError;
use crate::source::{AttachmentLocator, ResolvedSource};

/// Standard location of the Messages database inside an iOS backup
/// (domain, relative path) - the on-device path this corresponds to is
/// `/var/mobile/Library/SMS/sms.db`.
const MESSAGES_DOMAIN: &str = "HomeDomain";
const MESSAGES_RELATIVE_PATH: &str = "Library/SMS/sms.db";

/// Open the backup at `path`, extract (decrypting if needed) its Messages
/// database into `work_dir`, and return a [`ResolvedSource`] for it.
pub fn resolve(
    path: &Path,
    password: Option<String>,
    work_dir: &Path,
) -> Result<ResolvedSource, IngestError> {
    let auth = match password {
        Some(password) => Authentication::Password(password),
        None => Authentication::None,
    };
    let backup = Backup::open(path, &auth)?;

    let entries = backup.entries()?;
    let messages_entry = entries
        .iter()
        .find(|entry| {
            entry.domain == MESSAGES_DOMAIN && entry.relative_path == MESSAGES_RELATIVE_PATH
        })
        .ok_or(IngestError::MessagesDatabaseNotFound)?;

    let chat_db_path = work_dir.join("chat.db");
    let bytes = read_entry(&backup, messages_entry)?;
    std::fs::write(&chat_db_path, bytes)?;

    let device_name = Some(backup.lockdown().device_name.clone());

    // Index MediaDomain entries (attachments) by their relative path once,
    // rather than re-scanning `entries` per attachment lookup.
    let attachments_by_relative_path: HashMap<String, BackupFileEntry> = entries
        .into_iter()
        .filter(|entry| entry.domain == "MediaDomain")
        .map(|entry| (entry.relative_path.clone(), entry))
        .collect();

    Ok(ResolvedSource::new(
        chat_db_path,
        device_name,
        Box::new(BackupAttachmentLocator {
            backup,
            attachments_by_relative_path,
        }),
    ))
}

/// Read one file's plaintext bytes out of a backup, decrypting only if the
/// backup is actually encrypted (matches `crabapple::Backup::decrypt_entry`'s
/// own precondition - it errors on an unencrypted backup).
fn read_entry(backup: &Backup, entry: &BackupFileEntry) -> Result<Vec<u8>, IngestError> {
    if backup.is_encrypted() {
        Ok(backup.decrypt_entry(entry)?)
    } else {
        Ok(std::fs::read(backup.backup_path.join(entry.source()))?)
    }
}

struct BackupAttachmentLocator {
    backup: Backup,
    attachments_by_relative_path: HashMap<String, BackupFileEntry>,
}

impl AttachmentLocator for BackupAttachmentLocator {
    fn resolve(&self, path: &str) -> Result<Vec<u8>, IngestError> {
        // `imessage-database` reports the on-device path
        // (e.g. "/var/mobile/Library/SMS/Attachments/ab/01/GUID/photo.heic");
        // Manifest.db keys attachments by their path relative to the
        // "Library/SMS/Attachments" root. Match on that common suffix
        // rather than assuming either side's exact prefix.
        let relative_path = path
            .split_once("Library/SMS/Attachments")
            .map(|(_, suffix)| format!("Library/SMS/Attachments{suffix}"))
            .unwrap_or_else(|| path.to_string());

        let entry = self
            .attachments_by_relative_path
            .get(&relative_path)
            .ok_or_else(|| IngestError::AttachmentNotFoundInBackup(path.to_string()))?;

        read_entry(&self.backup, entry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use plist::{Dictionary, Integer, Uid, Value};
    use rusqlite::Connection;

    /// Minimal `Manifest.plist` for an unencrypted backup: just the fields
    /// `ManifestData::from_plist` actually requires.
    fn manifest_plist_bytes() -> Vec<u8> {
        let mut lockdown = Dictionary::new();
        lockdown.insert("BuildVersion".into(), Value::String("22B83".into()));
        lockdown.insert("DeviceName".into(), Value::String("Test iPhone".into()));
        lockdown.insert("ProductType".into(), Value::String("iPhone14,5".into()));
        lockdown.insert("ProductVersion".into(), Value::String("18.0".into()));
        lockdown.insert("SerialNumber".into(), Value::String("SN123456".into()));
        lockdown.insert(
            "UniqueDeviceID".into(),
            Value::String("00008030-000000000000000".into()),
        );

        let mut root = Dictionary::new();
        root.insert("IsEncrypted".into(), Value::Boolean(false));
        root.insert("Lockdown".into(), Value::Dictionary(lockdown));
        root.insert("Applications".into(), Value::Dictionary(Dictionary::new()));

        let mut bytes = Vec::new();
        Value::Dictionary(root)
            .to_writer_binary(&mut bytes)
            .unwrap();
        bytes
    }

    /// A minimal NSKeyedArchiver-encoded `MBFile` blob - the `$top`/
    /// `$objects`/`Uid` structure `MBFile::from_plist` walks - for an
    /// unencrypted file entry of `size` bytes. Real backups add
    /// `$archiver`/`$version` keys and an `EncryptionKey` entry; neither is
    /// read by `MBFile::from_plist` for an unencrypted file, so both are
    /// omitted here.
    fn mbfile_plist_bytes(size: u64) -> Vec<u8> {
        let mut file_dict = Dictionary::new();
        file_dict.insert("LastModified".into(), Value::Integer(Integer::from(0u64)));
        file_dict.insert("Flags".into(), Value::Integer(Integer::from(1u64)));
        file_dict.insert("GroupID".into(), Value::Integer(Integer::from(0i64)));
        file_dict.insert(
            "LastStatusChange".into(),
            Value::Integer(Integer::from(0u64)),
        );
        file_dict.insert("Birth".into(), Value::Integer(Integer::from(0u64)));
        file_dict.insert("Size".into(), Value::Integer(Integer::from(size)));
        file_dict.insert("Mode".into(), Value::Integer(Integer::from(0o100_644_u64)));
        file_dict.insert("InodeNumber".into(), Value::Integer(Integer::from(1u64)));
        file_dict.insert(
            "ProtectionClass".into(),
            Value::Integer(Integer::from(0u64)),
        );

        // Index 0 is NSKeyedArchiver's conventional "$null" placeholder;
        // the real object lives at index 1, referenced by the Uid below.
        let objects = vec![Value::String("$null".into()), Value::Dictionary(file_dict)];

        let mut top = Dictionary::new();
        top.insert("root".into(), Value::Uid(Uid::new(1)));

        let mut archive = Dictionary::new();
        archive.insert("$top".into(), Value::Dictionary(top));
        archive.insert("$objects".into(), Value::Array(objects));

        let mut bytes = Vec::new();
        Value::Dictionary(archive)
            .to_writer_binary(&mut bytes)
            .unwrap();
        bytes
    }

    /// Builds a synthetic, minimal, unencrypted backup directory: a real
    /// `Manifest.plist` + `Manifest.db` crabapple can open, with one
    /// `HomeDomain`/`Library/SMS/sms.db` entry and one `MediaDomain`
    /// attachment entry, both stored as plain (unencrypted) files.
    fn build_synthetic_backup(dir: &Path, sms_bytes: &[u8], attachment_bytes: &[u8]) {
        std::fs::write(dir.join("Manifest.plist"), manifest_plist_bytes()).unwrap();

        let db = Connection::open(dir.join("Manifest.db")).unwrap();
        db.execute_batch(
            "CREATE TABLE Files (fileID TEXT, domain TEXT, relativePath TEXT, flags INTEGER, file BLOB)",
        )
        .unwrap();

        let sms_id = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        db.execute(
            "INSERT INTO Files (fileID, domain, relativePath, flags, file) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                sms_id,
                "HomeDomain",
                "Library/SMS/sms.db",
                1,
                mbfile_plist_bytes(sms_bytes.len() as u64)
            ],
        )
        .unwrap();

        let attachment_id = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        db.execute(
            "INSERT INTO Files (fileID, domain, relativePath, flags, file) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                attachment_id,
                "MediaDomain",
                "Library/SMS/Attachments/ab/01/GUID/photo.heic",
                1,
                mbfile_plist_bytes(attachment_bytes.len() as u64)
            ],
        )
        .unwrap();
        drop(db);

        for (id, bytes) in [(sms_id, sms_bytes), (attachment_id, attachment_bytes)] {
            let subdir = dir.join(&id[0..2]);
            std::fs::create_dir_all(&subdir).unwrap();
            std::fs::write(subdir.join(id), bytes).unwrap();
        }
    }

    #[test]
    fn resolves_messages_db_and_attachments_from_an_unencrypted_backup() {
        let backup_dir = tempfile::tempdir().unwrap();
        let work_dir = tempfile::tempdir().unwrap();

        build_synthetic_backup(backup_dir.path(), b"fake sqlite bytes", b"fake heic bytes");

        let resolved = resolve(backup_dir.path(), None, work_dir.path()).unwrap();

        assert_eq!(resolved.device_name.as_deref(), Some("Test iPhone"));
        assert_eq!(
            std::fs::read(&resolved.chat_db_path).unwrap(),
            b"fake sqlite bytes"
        );

        // Real on-device attachment paths look like this; the locator
        // matches on the "Library/SMS/Attachments/..." suffix regardless
        // of the exact prefix imessage-database reports.
        let attachment_bytes = resolved
            .attachment_bytes("/var/mobile/Library/SMS/Attachments/ab/01/GUID/photo.heic")
            .unwrap();
        assert_eq!(attachment_bytes, b"fake heic bytes");
    }

    #[test]
    fn unknown_attachment_path_is_a_clear_error() {
        let backup_dir = tempfile::tempdir().unwrap();
        let work_dir = tempfile::tempdir().unwrap();
        build_synthetic_backup(backup_dir.path(), b"fake sqlite bytes", b"fake heic bytes");

        let resolved = resolve(backup_dir.path(), None, work_dir.path()).unwrap();
        let result = resolved.attachment_bytes("/var/mobile/Library/SMS/Attachments/nope.heic");

        assert!(matches!(
            result,
            Err(IngestError::AttachmentNotFoundInBackup(_))
        ));
    }

    #[test]
    fn missing_messages_entry_is_a_clear_error() {
        let backup_dir = tempfile::tempdir().unwrap();
        let work_dir = tempfile::tempdir().unwrap();

        std::fs::write(
            backup_dir.path().join("Manifest.plist"),
            manifest_plist_bytes(),
        )
        .unwrap();
        let db = Connection::open(backup_dir.path().join("Manifest.db")).unwrap();
        db.execute_batch(
            "CREATE TABLE Files (fileID TEXT, domain TEXT, relativePath TEXT, flags INTEGER, file BLOB)",
        )
        .unwrap();
        drop(db);

        let result = resolve(backup_dir.path(), None, work_dir.path());
        assert!(matches!(result, Err(IngestError::MessagesDatabaseNotFound)));
    }
}
