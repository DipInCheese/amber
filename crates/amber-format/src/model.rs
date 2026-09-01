// SPDX-License-Identifier: GPL-3.0-or-later
//! Input records the writer accepts, and output records the reader returns.
//!
//! These are independent of `amber-core`'s `DomainMessage`: `amber-format`
//! knows how to build and read a `.amber` archive from data shaped like
//! `SPEC.md`'s tables, regardless of where that data came from. Wiring
//! `amber-core`'s parser output into this shape is an ingest-time (M5)
//! concern.

use std::path::PathBuf;

// MARK: Writer input

/// Everything needed to write one conversation to a `.amber` archive.
#[derive(Debug, Clone)]
pub struct ArchiveInput {
    /// Free-text generator identifier, e.g. `"Amber 0.1.0"`.
    pub generator: String,
    pub source: SourceInput,
    pub conversation: ConversationInput,
    pub participants: Vec<ParticipantInput>,
    /// In the order they should be stored; the reader returns them ordered
    /// by `ts_unix_ms` regardless of input order.
    pub messages: Vec<MessageInput>,
    pub attachments: Vec<AttachmentInput>,
    pub reactions: Vec<ReactionInput>,
}

#[derive(Debug, Clone)]
pub struct SourceInput {
    /// One of `"device-backup"`, `"ios-backup"`, `"macos-live"`.
    pub source_type: String,
    pub device_name: Option<String>,
    pub ios_version: Option<String>,
    /// `(from, to)` as RFC 3339 timestamps.
    pub exported_range: Option<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct ConversationInput {
    pub chat_identifier: String,
    pub display_name: Option<String>,
    pub is_group: bool,
}

#[derive(Debug, Clone)]
pub struct ParticipantInput {
    /// Stable key other records reference this participant by; not stored
    /// verbatim unless it happens to equal `identifier` in the DB (it is
    /// the DB `identifier` column's value).
    pub identifier: String,
    pub display_name: Option<String>,
    pub is_me: bool,
}

#[derive(Debug, Clone)]
pub struct MessageInput {
    pub guid: String,
    /// Must match a [`ParticipantInput::identifier`] in the same
    /// [`ArchiveInput`].
    pub sender_identifier: String,
    pub is_from_me: bool,
    pub ts_unix_ms: i64,
    pub service: Option<String>,
    pub text: Option<String>,
    pub reply_to_guid: Option<String>,
    pub is_edited: bool,
    pub is_unsent: bool,
    /// Prior message texts, oldest first; serialized as a JSON array into
    /// `message.edit_history`.
    pub edit_history: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct AttachmentInput {
    /// Must match a [`MessageInput::guid`] in the same [`ArchiveInput`].
    pub message_guid: String,
    /// File on disk to read, hash, and embed content-addressed. Identical
    /// bytes across attachments are stored once in the archive.
    pub source_path: PathBuf,
    pub mime_type: Option<String>,
    /// Original transfer filename; defaults to `source_path`'s file name.
    pub filename: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub duration_ms: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct ReactionInput {
    pub target_message_guid: String,
    /// Must match a [`ParticipantInput::identifier`] in the same
    /// [`ArchiveInput`].
    pub participant_identifier: String,
    /// One of `love`, `like`, `dislike`, `laugh`, `emphasize`, `question`,
    /// `emoji`.
    pub kind: String,
    pub emoji: Option<String>,
    pub ts_unix_ms: Option<i64>,
    pub is_removed: bool,
}

// MARK: Reader output

#[derive(Debug, Clone, PartialEq)]
pub struct ParticipantRecord {
    pub id: i64,
    pub identifier: String,
    pub display_name: Option<String>,
    pub is_me: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MessageRecord {
    pub id: i64,
    pub guid: Option<String>,
    pub participant_id: Option<i64>,
    pub is_from_me: bool,
    pub ts_unix_ms: i64,
    pub service: Option<String>,
    pub text: Option<String>,
    pub reply_to_guid: Option<String>,
    pub is_edited: bool,
    pub is_unsent: bool,
    pub edit_history: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AttachmentRecord {
    pub id: i64,
    pub message_id: i64,
    pub sha256: String,
    pub rel_path: String,
    pub mime_type: Option<String>,
    pub filename: Option<String>,
    pub byte_size: Option<i64>,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub duration_ms: Option<i64>,
    pub thumb_path: Option<String>,
}

/// One row of the `day_index` scrubber cache (`SPEC.md` §3).
#[derive(Debug, Clone, PartialEq)]
pub struct DayBucketRecord {
    /// `YYYY-MM-DD`, UTC.
    pub day: String,
    pub message_count: i64,
    pub first_msg_id: Option<i64>,
    pub last_msg_id: Option<i64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReactionRecord {
    pub id: i64,
    pub target_message_guid: String,
    pub participant_id: Option<i64>,
    pub kind: String,
    pub emoji: Option<String>,
    pub ts_unix_ms: Option<i64>,
    pub is_removed: bool,
}
