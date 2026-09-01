// SPDX-License-Identifier: GPL-3.0-or-later
//! Builds a `.amber` archive from an [`ArchiveInput`].

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::File;
use std::io::Write;
use std::path::Path;

use chrono::{TimeZone, Utc};
use rusqlite::Connection;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use crate::error::AmberFormatError;
use crate::hash::sha256_hex;
use crate::manifest::{
    Manifest, ManifestConversation, ManifestCounts, ManifestExportedRange, ManifestIntegrity,
    ManifestParticipant, ManifestSource, FORMAT_VERSION,
};
use crate::model::ArchiveInput;
use crate::schema::{APPLICATION_ID, SCHEMA_SQL, SCHEMA_VERSION};

/// Write `input` as a `.amber` archive at `output_path`, overwriting any
/// existing file.
pub fn write_archive(output_path: &Path, input: &ArchiveInput) -> Result<(), AmberFormatError> {
    let db_bytes = build_messages_db(input)?;
    let db_sha256 = sha256_hex(&db_bytes);

    let unique_attachments = dedupe_attachments(input)?;

    let manifest = build_manifest(input, &db_sha256)?;
    let manifest_json = serde_json::to_vec_pretty(&manifest)?;

    let zip_file = File::create(output_path)?;
    let mut zip = ZipWriter::new(zip_file);

    let compressible = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);

    zip.start_file("manifest.json", compressible)?;
    zip.write_all(&manifest_json)?;

    zip.start_file("messages.db", compressible)?;
    zip.write_all(&db_bytes)?;

    for (rel_path, bytes) in &unique_attachments {
        zip.start_file(rel_path, stored)?;
        zip.write_all(bytes)?;
    }

    zip.finish()?;
    Ok(())
}

/// Read and hash every attachment's source file once, keyed by the archive
/// path it will be stored at, so identical media is embedded a single time
/// even if it's attached to multiple messages.
fn dedupe_attachments(input: &ArchiveInput) -> Result<BTreeMap<String, Vec<u8>>, AmberFormatError> {
    let mut by_rel_path = BTreeMap::new();
    for attachment in &input.attachments {
        let (rel_path, bytes) = read_and_address_attachment(&attachment.source_path)?;
        by_rel_path.entry(rel_path).or_insert(bytes);
    }
    Ok(by_rel_path)
}

fn read_and_address_attachment(source_path: &Path) -> Result<(String, Vec<u8>), AmberFormatError> {
    let bytes = std::fs::read(source_path)
        .map_err(|_| AmberFormatError::AttachmentSourceMissing(source_path.to_path_buf()))?;
    let sha256 = sha256_hex(&bytes);
    let ext = source_path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("bin");
    let prefix = &sha256[0..2];
    Ok((format!("attachments/{prefix}/{sha256}.{ext}"), bytes))
}

fn build_manifest(input: &ArchiveInput, db_sha256: &str) -> Result<Manifest, AmberFormatError> {
    Ok(Manifest {
        format: "amber-archive".to_string(),
        format_version: FORMAT_VERSION.to_string(),
        schema_version: SCHEMA_VERSION,
        created_utc: Utc::now().to_rfc3339(),
        generator: input.generator.clone(),
        source: ManifestSource {
            source_type: input.source.source_type.clone(),
            device_name: input.source.device_name.clone(),
            ios_version: input.source.ios_version.clone(),
            exported_range: input.source.exported_range.as_ref().map(|(from, to)| {
                ManifestExportedRange {
                    from: from.clone(),
                    to: to.clone(),
                }
            }),
        },
        conversation: ManifestConversation {
            chat_identifier: input.conversation.chat_identifier.clone(),
            display_name: input.conversation.display_name.clone(),
            is_group: input.conversation.is_group,
            participants: input
                .participants
                .iter()
                .map(|p| ManifestParticipant {
                    identifier: p.identifier.clone(),
                    display_name: p.display_name.clone(),
                    is_me: p.is_me,
                })
                .collect(),
        },
        counts: ManifestCounts {
            messages: input.messages.len() as i64,
            attachments: input.attachments.len() as i64,
        },
        integrity: ManifestIntegrity {
            messages_db_sha256: db_sha256.to_string(),
            attachments_sha256_indexed: true,
        },
    })
}

/// Build `messages.db` in a temp file (SQLite needs a real file path, not
/// just bytes in memory) and return its final on-disk bytes.
fn build_messages_db(input: &ArchiveInput) -> Result<Vec<u8>, AmberFormatError> {
    let temp_file = tempfile::NamedTempFile::new()?;
    let temp_path = temp_file.into_temp_path();

    {
        let db = Connection::open(&temp_path)?;
        db.pragma_update(None, "application_id", APPLICATION_ID)?;
        db.pragma_update(None, "user_version", SCHEMA_VERSION)?;
        db.execute_batch(SCHEMA_SQL)?;

        let participant_ids = insert_participants(&db, input)?;
        let message_ids = insert_messages(&db, input, &participant_ids)?;
        insert_attachments(&db, input, &message_ids)?;
        insert_reactions(&db, input, &participant_ids)?;
        rebuild_fts(&db)?;
        build_day_index(&db)?;
    }

    let bytes = std::fs::read(&temp_path)?;
    Ok(bytes)
}

fn insert_participants(
    db: &Connection,
    input: &ArchiveInput,
) -> Result<HashMap<String, i64>, AmberFormatError> {
    let mut ids = HashMap::new();
    let mut stmt = db
        .prepare("INSERT INTO participant (identifier, display_name, is_me) VALUES (?1, ?2, ?3)")?;
    for participant in &input.participants {
        stmt.execute((
            &participant.identifier,
            &participant.display_name,
            participant.is_me,
        ))?;
        ids.insert(participant.identifier.clone(), db.last_insert_rowid());
    }
    Ok(ids)
}

fn insert_messages(
    db: &Connection,
    input: &ArchiveInput,
    participant_ids: &HashMap<String, i64>,
) -> Result<HashMap<String, i64>, AmberFormatError> {
    let mut ids = HashMap::new();
    let mut stmt = db.prepare(
        "INSERT INTO message
            (guid, participant_id, is_from_me, ts_unix_ms, service, text,
             reply_to_guid, is_edited, is_unsent, edit_history)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
    )?;
    for message in &input.messages {
        let participant_id = participant_ids
            .get(&message.sender_identifier)
            .ok_or_else(|| {
                AmberFormatError::UnknownSender(
                    message.guid.clone(),
                    message.sender_identifier.clone(),
                )
            })?;
        let edit_history_json = match &message.edit_history {
            Some(history) => Some(serde_json::to_string(history)?),
            None => None,
        };

        stmt.execute((
            &message.guid,
            participant_id,
            message.is_from_me,
            message.ts_unix_ms,
            &message.service,
            &message.text,
            &message.reply_to_guid,
            message.is_edited,
            message.is_unsent,
            edit_history_json,
        ))?;
        ids.insert(message.guid.clone(), db.last_insert_rowid());
    }
    Ok(ids)
}

fn insert_attachments(
    db: &Connection,
    input: &ArchiveInput,
    message_ids: &HashMap<String, i64>,
) -> Result<(), AmberFormatError> {
    let mut stmt = db.prepare(
        "INSERT INTO attachment
            (message_id, sha256, rel_path, mime_type, filename, byte_size, width, height, duration_ms)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
    )?;
    for attachment in &input.attachments {
        let message_id = message_ids.get(&attachment.message_guid).ok_or_else(|| {
            AmberFormatError::UnknownSender(
                attachment.message_guid.clone(),
                "<attachment target message>".to_string(),
            )
        })?;
        let (rel_path, bytes) = read_and_address_attachment(&attachment.source_path)?;
        let sha256 = sha256_hex(&bytes);
        let filename = attachment.filename.clone().or_else(|| {
            attachment
                .source_path
                .file_name()
                .and_then(|name| name.to_str())
                .map(str::to_string)
        });

        stmt.execute((
            message_id,
            &sha256,
            &rel_path,
            &attachment.mime_type,
            &filename,
            bytes.len() as i64,
            attachment.width,
            attachment.height,
            attachment.duration_ms,
        ))?;
    }
    Ok(())
}

fn insert_reactions(
    db: &Connection,
    input: &ArchiveInput,
    participant_ids: &HashMap<String, i64>,
) -> Result<(), AmberFormatError> {
    let mut stmt = db.prepare(
        "INSERT INTO reaction
            (target_message_guid, participant_id, kind, emoji, ts_unix_ms, is_removed)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )?;
    for reaction in &input.reactions {
        let participant_id = participant_ids
            .get(&reaction.participant_identifier)
            .ok_or_else(|| {
                AmberFormatError::UnknownReactionParticipant(
                    reaction.target_message_guid.clone(),
                    reaction.participant_identifier.clone(),
                )
            })?;

        stmt.execute((
            &reaction.target_message_guid,
            participant_id,
            &reaction.kind,
            &reaction.emoji,
            reaction.ts_unix_ms,
            reaction.is_removed,
        ))?;
    }
    Ok(())
}

/// `message_fts` is an external-content FTS5 table (`content='message'`),
/// so it isn't kept in sync automatically without triggers. A one-shot
/// rebuild after loading `message` is the simplest way to populate it for a
/// single-pass writer.
fn rebuild_fts(db: &Connection) -> Result<(), AmberFormatError> {
    db.execute(
        "INSERT INTO message_fts(message_fts) VALUES ('rebuild')",
        (),
    )?;
    Ok(())
}

fn build_day_index(db: &Connection) -> Result<(), AmberFormatError> {
    let mut select = db.prepare("SELECT id, ts_unix_ms FROM message ORDER BY ts_unix_ms")?;
    let rows = select.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)))?;

    // day -> (count, first_msg_id, last_msg_id)
    let mut days: BTreeMap<String, (i64, i64, i64)> = BTreeMap::new();
    let mut seen_days_in_order: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    for row in rows {
        let (id, ts_unix_ms) = row?;
        let day = day_bucket(ts_unix_ms);

        if seen.insert(day.clone()) {
            seen_days_in_order.push(day.clone());
        }

        days.entry(day)
            .and_modify(|(count, _first, last)| {
                *count += 1;
                *last = id;
            })
            .or_insert((1, id, id));
    }

    let mut insert = db.prepare(
        "INSERT INTO day_index (day, message_count, first_msg_id, last_msg_id)
         VALUES (?1, ?2, ?3, ?4)",
    )?;
    for day in seen_days_in_order {
        let (count, first, last) = days[&day];
        insert.execute((&day, count, first, last))?;
    }

    Ok(())
}

/// UTC calendar day (`YYYY-MM-DD`) containing a Unix-millisecond timestamp.
fn day_bucket(ts_unix_ms: i64) -> String {
    Utc.timestamp_millis_opt(ts_unix_ms)
        .single()
        .map(|dt| dt.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "1970-01-01".to_string())
}
