// SPDX-License-Identifier: GPL-3.0-or-later
//! Lists the conversations available in a Messages database, so a caller
//! can let the user pick one before parsing it in full (`parse_conversation`
//! needs a specific `chat_identifier`).

use std::path::Path;

use imessage_database::tables::table::get_connection;
use serde::{Deserialize, Serialize};

use crate::error::AmberCoreError;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConversationSummary {
    pub chat_identifier: String,
    pub display_name: Option<String>,
    /// More than one other participant in `chat_handle_join`.
    pub is_group: bool,
    pub message_count: i64,
}

/// List every conversation in the database at `db_path`, most active first.
pub fn list_conversations(db_path: &Path) -> Result<Vec<ConversationSummary>, AmberCoreError> {
    let db = get_connection(db_path)?;

    let mut statement = db.prepare(
        "SELECT
             c.chat_identifier,
             c.display_name,
             (SELECT COUNT(*) FROM chat_message_join m WHERE m.chat_id = c.ROWID) AS message_count,
             (SELECT COUNT(*) FROM chat_handle_join h WHERE h.chat_id = c.ROWID) AS participant_count
         FROM chat c
         ORDER BY message_count DESC",
    )?;

    let rows = statement.query_map([], |row| {
        let display_name: Option<String> = row.get(1)?;
        Ok(ConversationSummary {
            chat_identifier: row.get(0)?,
            display_name: display_name.filter(|name| !name.is_empty()),
            message_count: row.get(2)?,
            is_group: row.get::<_, i64>(3)? > 1,
        })
    })?;

    rows.collect::<Result<_, _>>().map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn build_fixture_db() -> tempfile::TempPath {
        let file = tempfile::NamedTempFile::new().unwrap();
        let path = file.into_temp_path();
        let db = Connection::open(&path).unwrap();
        db.execute_batch(
            "
            CREATE TABLE message (ROWID INTEGER PRIMARY KEY, guid TEXT UNIQUE, date INTEGER);
            CREATE TABLE chat (
                ROWID INTEGER PRIMARY KEY,
                chat_identifier TEXT NOT NULL,
                service_name TEXT,
                display_name TEXT
            );
            CREATE TABLE chat_message_join (chat_id INTEGER, message_id INTEGER);
            CREATE TABLE chat_handle_join (chat_id INTEGER, handle_id INTEGER);

            -- A 1:1 conversation with 2 messages.
            INSERT INTO chat (ROWID, chat_identifier, display_name) VALUES (1, '+15551234567', NULL);
            INSERT INTO chat_handle_join (chat_id, handle_id) VALUES (1, 1);
            INSERT INTO message (ROWID, guid, date) VALUES (1, 'm1', 0), (2, 'm2', 0);
            INSERT INTO chat_message_join (chat_id, message_id) VALUES (1, 1), (1, 2);

            -- A group conversation with 1 message and 3 other participants.
            INSERT INTO chat (ROWID, chat_identifier, display_name) VALUES (2, 'chat123456', 'Family');
            INSERT INTO chat_handle_join (chat_id, handle_id) VALUES (2, 2), (2, 3), (2, 4);
            INSERT INTO message (ROWID, guid, date) VALUES (3, 'm3', 0);
            INSERT INTO chat_message_join (chat_id, message_id) VALUES (2, 3);
            ",
        )
        .unwrap();
        path
    }

    #[test]
    fn lists_conversations_most_active_first_with_group_detection() {
        let db_path = build_fixture_db();

        let conversations = list_conversations(&db_path).unwrap();

        assert_eq!(conversations.len(), 2);
        assert_eq!(conversations[0].chat_identifier, "+15551234567");
        assert_eq!(conversations[0].message_count, 2);
        assert!(!conversations[0].is_group);

        assert_eq!(conversations[1].chat_identifier, "chat123456");
        assert_eq!(conversations[1].display_name.as_deref(), Some("Family"));
        assert_eq!(conversations[1].message_count, 1);
        assert!(conversations[1].is_group);
    }
}
