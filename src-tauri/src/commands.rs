// SPDX-License-Identifier: GPL-3.0-or-later
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use amber_format::AmberArchive;
use tauri::State;

use crate::dto::{
    AttachmentDto, DayBucketDto, MessageDto, OpenArchiveResult, ParticipantDto, ReactionDto,
};

/// The currently opened archive, if any. Amber's viewer looks at one
/// conversation at a time, so a single slot (rather than a map of open
/// archives) is enough for M3.
#[derive(Default)]
pub struct AppState {
    archive: Mutex<Option<AmberArchive>>,
}

impl AppState {
    /// Used by the `amber-attachment` protocol handler to reach the open
    /// archive without going through a `#[tauri::command]`.
    pub(crate) fn archive(&self) -> MutexGuard<'_, Option<AmberArchive>> {
        self.archive.lock().unwrap()
    }
}

#[tauri::command]
pub fn open_archive(path: String, state: State<AppState>) -> Result<OpenArchiveResult, String> {
    open_and_store(Path::new(&path), &state)
}

/// Opens `path` and makes it the current archive, returning its summary.
/// Shared by the [`open_archive`] command and `ingest::ingest_conversation`
/// (which writes a fresh `.amber` and then opens it the exact same way).
pub(crate) fn open_and_store(path: &Path, state: &AppState) -> Result<OpenArchiveResult, String> {
    let archive = AmberArchive::open(path).map_err(|err| err.to_string())?;
    let manifest = archive.manifest();

    let result = OpenArchiveResult {
        chat_identifier: manifest.conversation.chat_identifier.clone(),
        display_name: manifest.conversation.display_name.clone(),
        is_group: manifest.conversation.is_group,
        participants: manifest
            .conversation
            .participants
            .iter()
            .map(|p| ParticipantDto {
                identifier: p.identifier.clone(),
                display_name: p.display_name.clone(),
                is_me: p.is_me,
            })
            .collect(),
        message_count: manifest.counts.messages,
    };

    *state.archive() = Some(archive);
    Ok(result)
}

/// All messages in the currently open archive, chronologically ordered,
/// with attachments and reactions nested under their message.
///
/// Loads the full conversation in one call. `SPEC.md`'s `.amber` archives
/// are single-conversation, so even a 7-year, tens-of-thousands-of-messages
/// history is a modest amount of JSON; the virtualized list (M4) is what
/// keeps *rendering* that cheap, not paginated fetching.
#[tauri::command]
pub fn query_messages(state: State<AppState>) -> Result<Vec<MessageDto>, String> {
    let guard = state.archive.lock().unwrap();
    let archive = guard.as_ref().ok_or("no archive is open")?;

    let participants_by_id: HashMap<i64, ParticipantDto> = archive
        .list_participants()
        .map_err(|err| err.to_string())?
        .into_iter()
        .map(|p| {
            (
                p.id,
                ParticipantDto {
                    identifier: p.identifier,
                    display_name: p.display_name,
                    is_me: p.is_me,
                },
            )
        })
        .collect();

    let mut attachments_by_message: HashMap<i64, Vec<AttachmentDto>> = HashMap::new();
    for attachment in archive.list_attachments().map_err(|err| err.to_string())? {
        attachments_by_message
            .entry(attachment.message_id)
            .or_default()
            .push(AttachmentDto {
                rel_path: attachment.rel_path,
                mime_type: attachment.mime_type,
                filename: attachment.filename,
                width: attachment.width,
                height: attachment.height,
                duration_ms: attachment.duration_ms,
            });
    }

    let mut reactions_by_target_guid: HashMap<String, Vec<ReactionDto>> = HashMap::new();
    for reaction in archive.list_reactions().map_err(|err| err.to_string())? {
        reactions_by_target_guid
            .entry(reaction.target_message_guid)
            .or_default()
            .push(ReactionDto {
                participant_identifier: reaction
                    .participant_id
                    .and_then(|id| participants_by_id.get(&id))
                    .map(|p| p.identifier.clone()),
                kind: reaction.kind,
                emoji: reaction.emoji,
                ts_unix_ms: reaction.ts_unix_ms,
                is_removed: reaction.is_removed,
            });
    }

    let messages = archive
        .list_messages()
        .map_err(|err| err.to_string())?
        .into_iter()
        .map(|message| {
            let reactions = message
                .guid
                .as_deref()
                .and_then(|guid| reactions_by_target_guid.get(guid))
                .cloned()
                .unwrap_or_default();

            MessageDto {
                sender_identifier: message
                    .participant_id
                    .and_then(|id| participants_by_id.get(&id))
                    .map(|p| p.identifier.clone()),
                attachments: attachments_by_message
                    .remove(&message.id)
                    .unwrap_or_default(),
                reactions,
                guid: message.guid,
                ts_unix_ms: message.ts_unix_ms,
                is_from_me: message.is_from_me,
                text: message.text,
                reply_to_guid: message.reply_to_guid,
                is_edited: message.is_edited,
                is_unsent: message.is_unsent,
                edit_history: message.edit_history,
            }
        })
        .collect();

    Ok(messages)
}

/// The `day_index` scrubber cache for the currently open archive.
#[tauri::command]
pub fn get_day_index(state: State<AppState>) -> Result<Vec<DayBucketDto>, String> {
    let guard = state.archive.lock().unwrap();
    let archive = guard.as_ref().ok_or("no archive is open")?;

    Ok(archive
        .list_day_buckets()
        .map_err(|err| err.to_string())?
        .into_iter()
        .map(|bucket| DayBucketDto {
            day: bucket.day,
            message_count: bucket.message_count,
        })
        .collect())
}
