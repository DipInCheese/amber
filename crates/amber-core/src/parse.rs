// SPDX-License-Identifier: GPL-3.0-or-later
//! Parses a single conversation out of a Messages `chat.db`-shaped SQLite
//! database into [`DomainMessage`]s.
//!
//! This module only knows how to read an already-resolved database file; it
//! has no opinion about where that file came from (a live Mac, a device
//! backup, or an iTunes backup are all `amber-ingest`'s concern, M5).

use std::collections::BTreeSet;
use std::path::Path;

use imessage_database::tables::attachment::Attachment;
use imessage_database::tables::chat::Chat;
use imessage_database::tables::handle::Handle;
use imessage_database::tables::messages::Message;
use imessage_database::tables::table::{get_connection, Cacheable, Table};
use imessage_database::util::query_context::QueryContext;

use crate::error::AmberCoreError;
use crate::model::DomainMessage;
use crate::time::apple_ts_to_unix_ms;

/// Parse every message in the conversation identified by `chat_identifier`
/// (a `chat.chat_identifier` value - typically a phone number, email, or
/// group chat identifier) out of the database at `db_path`, in chronological
/// order.
///
/// Returns [`AmberCoreError::ConversationNotFound`] if no chat row matches.
pub fn parse_conversation(
    db_path: &Path,
    chat_identifier: &str,
) -> Result<Vec<DomainMessage>, AmberCoreError> {
    let db = get_connection(db_path)?;

    let chats = Chat::cache(&db)?;
    let matching_chat_ids: BTreeSet<i32> = chats
        .values()
        .filter(|chat| chat.chat_identifier == chat_identifier)
        .map(|chat| chat.rowid)
        .collect();

    if matching_chat_ids.is_empty() {
        return Err(AmberCoreError::ConversationNotFound(
            chat_identifier.to_string(),
        ));
    }

    let handles = Handle::cache(&db)?;

    let mut context = QueryContext::default();
    context.set_selected_chat_ids(matching_chat_ids);

    let mut statement = Message::stream_rows(&db, &context)?;
    let mut messages = Vec::new();

    for message in Message::rows(&mut statement, [])? {
        let mut message = message?;

        // Decode `attributedBody` (typedstream) into plain text when the
        // legacy `text` column is empty - this is the modern-format case for
        // the vast majority of messages sent since iOS 11 / macOS Big Sur.
        if let Ok(body) = message.parse_body(&db) {
            message.apply_body(body);
        }

        let attachment_paths = Attachment::from_message(&db, &message)?
            .into_iter()
            .filter_map(|attachment| attachment.filename().map(str::to_string))
            .collect();

        let sender = if message.is_from_me() {
            "me".to_string()
        } else {
            message
                .handle_id
                .and_then(|id| handles.get(&id))
                .cloned()
                .unwrap_or_else(|| "unknown".to_string())
        };

        messages.push(DomainMessage {
            guid: message.guid.clone(),
            text: message.text.clone(),
            ts_unix_ms: apple_ts_to_unix_ms(message.date),
            sender,
            is_from_me: message.is_from_me(),
            attachment_paths,
        });
    }

    Ok(messages)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use std::fs;

    /// A real `attributedBody` typedstream BLOB that decodes to "Noter test".
    /// See `fixtures/typedstream/README.md` for provenance.
    fn attributed_body_text_only_fixture() -> Vec<u8> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/typedstream/AttributedBodyTextOnly.bin");
        fs::read(path).expect("typedstream fixture should be present")
    }

    /// Build a synthetic, minimal `chat.db`-shaped SQLite database on disk
    /// (a real file is required: `get_connection` rejects anything that
    /// isn't an existing file, and `imessage-database` opens it read-only).
    ///
    /// Schema matches the macOS Ventura+ / iOS 16+ shape so `Message::get`
    /// resolves via its first (explicit-columns) query variant.
    fn build_fixture_db() -> tempfile::TempPath {
        let file = tempfile::NamedTempFile::new().unwrap();
        let path = file.into_temp_path();

        // `get_connection` opens read-only, so build the schema and data
        // with a separate read-write connection first.
        let db = Connection::open(&path).unwrap();
        db.execute_batch(
            "
            CREATE TABLE message (
                ROWID INTEGER PRIMARY KEY,
                guid TEXT UNIQUE NOT NULL,
                text TEXT,
                service TEXT,
                handle_id INTEGER,
                destination_caller_id TEXT,
                subject TEXT,
                date INTEGER NOT NULL,
                date_read INTEGER,
                date_delivered INTEGER,
                is_from_me INTEGER NOT NULL DEFAULT 0,
                is_read INTEGER,
                item_type INTEGER,
                other_handle INTEGER,
                share_status INTEGER,
                share_direction INTEGER,
                group_title TEXT,
                group_action_type INTEGER,
                associated_message_guid TEXT,
                associated_message_type INTEGER,
                balloon_bundle_id TEXT,
                expressive_send_style_id TEXT,
                thread_originator_guid TEXT,
                thread_originator_part TEXT,
                date_edited INTEGER,
                associated_message_emoji TEXT,
                attributedBody BLOB,
                message_summary_info BLOB,
                payload_data BLOB
            );
            CREATE TABLE chat (
                ROWID INTEGER PRIMARY KEY,
                chat_identifier TEXT NOT NULL,
                service_name TEXT,
                display_name TEXT
            );
            CREATE TABLE handle (
                ROWID INTEGER PRIMARY KEY,
                id TEXT NOT NULL,
                person_centric_id TEXT
            );
            CREATE TABLE chat_message_join (
                chat_id INTEGER,
                message_id INTEGER
            );
            CREATE TABLE chat_handle_join (
                chat_id INTEGER,
                handle_id INTEGER
            );
            CREATE TABLE message_attachment_join (
                message_id INTEGER,
                attachment_id INTEGER
            );
            CREATE TABLE attachment (
                ROWID INTEGER PRIMARY KEY,
                guid TEXT,
                filename TEXT,
                uti TEXT,
                mime_type TEXT,
                transfer_name TEXT,
                total_bytes INTEGER,
                is_sticker INTEGER,
                hide_attachment INTEGER,
                emoji_description TEXT
            );
            CREATE TABLE chat_recoverable_message_join (
                chat_id INTEGER,
                message_id INTEGER
            );

            INSERT INTO chat (ROWID, chat_identifier, service_name, display_name)
                VALUES (1, '+15551234567', 'iMessage', NULL);
            INSERT INTO handle (ROWID, id) VALUES (1, '+15551234567');
            ",
        )
        .unwrap();

        // Message 1: plain `text`, from the other participant.
        db.execute(
            "INSERT INTO message
                (ROWID, guid, text, handle_id, date, is_from_me, associated_message_type)
             VALUES (1, 'msg-1', 'hey there', 1, 700000000000000000, 0, 0)",
            [],
        )
        .unwrap();
        db.execute(
            "INSERT INTO chat_message_join (chat_id, message_id) VALUES (1, 1)",
            [],
        )
        .unwrap();

        // Message 2: empty `text`, real body only in `attributedBody` - from
        // the database owner. Proves attributedBody decoding end-to-end.
        db.execute(
            "INSERT INTO message
                (ROWID, guid, text, handle_id, date, is_from_me, attributedBody, associated_message_type)
             VALUES (2, 'msg-2', NULL, NULL, 700000001000000000, 1, ?1, 0)",
            [attributed_body_text_only_fixture()],
        )
        .unwrap();
        db.execute(
            "INSERT INTO chat_message_join (chat_id, message_id) VALUES (1, 2)",
            [],
        )
        .unwrap();

        path
    }

    #[test]
    fn parses_text_and_attributed_body_messages_in_order() {
        let db_path = build_fixture_db();

        let messages = parse_conversation(&db_path, "+15551234567").unwrap();

        assert_eq!(messages.len(), 2);

        assert_eq!(messages[0].guid, "msg-1");
        assert_eq!(messages[0].text.as_deref(), Some("hey there"));
        assert!(!messages[0].is_from_me);
        assert_eq!(messages[0].sender, "+15551234567");
        assert_eq!(
            messages[0].ts_unix_ms,
            apple_ts_to_unix_ms(700000000000000000)
        );

        // The whole point of this fixture: `text` was NULL in the row, so
        // this text only exists because `attributedBody` was decoded.
        assert_eq!(messages[1].guid, "msg-2");
        assert_eq!(messages[1].text.as_deref(), Some("Noter test"));
        assert!(messages[1].is_from_me);
        assert_eq!(messages[1].sender, "me");
    }

    #[test]
    fn unknown_conversation_is_an_error() {
        let db_path = build_fixture_db();

        let result = parse_conversation(&db_path, "+15559999999");

        assert!(matches!(
            result,
            Err(AmberCoreError::ConversationNotFound(id)) if id == "+15559999999"
        ));
    }
}
