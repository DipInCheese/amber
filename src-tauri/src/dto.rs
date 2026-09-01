// SPDX-License-Identifier: GPL-3.0-or-later
//! JSON-serializable shapes sent to the frontend. Deliberately distinct from
//! `amber_format`'s reader records: these are pre-joined (attachments and
//! reactions nested under their message) for direct use by the viewer.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ParticipantDto {
    pub identifier: String,
    pub display_name: Option<String>,
    pub is_me: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AttachmentDto {
    pub rel_path: String,
    pub mime_type: Option<String>,
    pub filename: Option<String>,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub duration_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReactionDto {
    pub participant_identifier: Option<String>,
    pub kind: String,
    pub emoji: Option<String>,
    pub ts_unix_ms: Option<i64>,
    pub is_removed: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct MessageDto {
    pub guid: Option<String>,
    pub ts_unix_ms: i64,
    pub sender_identifier: Option<String>,
    pub is_from_me: bool,
    pub text: Option<String>,
    pub reply_to_guid: Option<String>,
    pub is_edited: bool,
    pub is_unsent: bool,
    pub edit_history: Option<Vec<String>>,
    pub attachments: Vec<AttachmentDto>,
    pub reactions: Vec<ReactionDto>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DayBucketDto {
    pub day: String,
    pub message_count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpenArchiveResult {
    pub chat_identifier: String,
    pub display_name: Option<String>,
    pub is_group: bool,
    pub participants: Vec<ParticipantDto>,
    pub message_count: i64,
}
