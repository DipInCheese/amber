// SPDX-License-Identifier: GPL-3.0-or-later
//! Lists the conversations available in a Messages database, so a caller
//! can let the user pick one before parsing it in full (`parse_conversation`
//! needs a specific `chat_identifier`).

use std::collections::HashMap;
use std::path::Path;

use imessage_database::tables::table::get_connection;
use serde::{Deserialize, Serialize};

use crate::error::AmberCoreError;
use crate::time::apple_ts_to_unix_ms;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConversationSummary {
    pub chat_identifier: String,
    pub display_name: Option<String>,
    /// More than one other participant in `chat_handle_join`.
    pub is_group: bool,
    pub message_count: i64,
    /// When the newest message in the conversation was sent/received (Unix
    /// ms), so a picker can show and sort by recency. `None` for an empty
    /// conversation or one whose messages all carry Apple's "unset" date.
    pub last_message_ts_unix_ms: Option<i64>,
    /// The other people in the conversation (phone numbers / email
    /// addresses, as stored by Messages), so a caller can show names for
    /// them - an unnamed group chat has no label of its own.
    #[serde(default)]
    pub participants: Vec<String>,
}

/// List every conversation in the database at `db_path`, most recently
/// active first (ties broken by message count).
pub fn list_conversations(db_path: &Path) -> Result<Vec<ConversationSummary>, AmberCoreError> {
    let db = get_connection(db_path)?;

    let mut participants_by_chat: HashMap<i64, Vec<String>> = HashMap::new();
    {
        let mut handles = db.prepare(
            "SELECT j.chat_id, h.id
               FROM chat_handle_join j
               JOIN handle h ON h.ROWID = j.handle_id
              ORDER BY j.chat_id, h.ROWID",
        )?;
        let pairs = handles.query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?;
        for pair in pairs {
            let (chat_id, handle) = pair?;
            participants_by_chat
                .entry(chat_id)
                .or_default()
                .push(handle);
        }
    }

    let mut statement = db.prepare(
        "SELECT
             c.chat_identifier,
             c.display_name,
             (SELECT COUNT(*) FROM chat_message_join m WHERE m.chat_id = c.ROWID) AS message_count,
             (SELECT COUNT(*) FROM chat_handle_join h WHERE h.chat_id = c.ROWID) AS participant_count,
             (SELECT MAX(msg.date)
                FROM chat_message_join j
                JOIN message msg ON msg.ROWID = j.message_id
               WHERE j.chat_id = c.ROWID) AS last_date,
             c.ROWID
         FROM chat c
         ORDER BY COALESCE(last_date, 0) DESC, message_count DESC",
    )?;

    let rows = statement.query_map([], |row| {
        let display_name: Option<String> = row.get(1)?;
        let last_date: Option<i64> = row.get(4)?;
        let chat_id: i64 = row.get(5)?;
        Ok(ConversationSummary {
            participants: participants_by_chat
                .get(&chat_id)
                .cloned()
                .unwrap_or_default(),
            chat_identifier: row.get(0)?,
            display_name: display_name.filter(|name| !name.is_empty()),
            message_count: row.get(2)?,
            is_group: row.get::<_, i64>(3)? > 1,
            last_message_ts_unix_ms: last_date.filter(|&raw| raw != 0).map(apple_ts_to_unix_ms),
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
            CREATE TABLE handle (ROWID INTEGER PRIMARY KEY, id TEXT);
            INSERT INTO handle (ROWID, id) VALUES
                (1, '+15551234567'), (2, '+15550000002'), (3, '+15550000003'), (4, 'x@example.com');

            -- A 1:1 conversation with 2 messages.
            INSERT INTO chat (ROWID, chat_identifier, display_name) VALUES (1, '+15551234567', NULL);
            INSERT INTO chat_handle_join (chat_id, handle_id) VALUES (1, 1);
            -- Dates are nanoseconds since the Apple epoch (2001-01-01).
            INSERT INTO message (ROWID, guid, date) VALUES
                (1, 'm1', 600000000000000000), (2, 'm2', 700000000000000000);
            INSERT INTO chat_message_join (chat_id, message_id) VALUES (1, 1), (1, 2);

            -- A group conversation with 1 message and 3 other participants,
            -- more recent than the 1:1 even though it has fewer messages.
            INSERT INTO chat (ROWID, chat_identifier, display_name) VALUES (2, 'chat123456', 'Family');
            INSERT INTO chat_handle_join (chat_id, handle_id) VALUES (2, 2), (2, 3), (2, 4);
            INSERT INTO message (ROWID, guid, date) VALUES (3, 'm3', 750000000000000000);
            INSERT INTO chat_message_join (chat_id, message_id) VALUES (2, 3);

            -- A conversation with no messages at all.
            INSERT INTO chat (ROWID, chat_identifier, display_name) VALUES (3, 'empty', NULL);
            ",
        )
        .unwrap();
        path
    }

    #[test]
    fn lists_conversations_most_recent_first_with_group_detection() {
        let db_path = build_fixture_db();

        let conversations = list_conversations(&db_path).unwrap();

        assert_eq!(conversations.len(), 3);

        assert_eq!(conversations[0].chat_identifier, "chat123456");
        assert_eq!(conversations[0].display_name.as_deref(), Some("Family"));
        assert_eq!(conversations[0].message_count, 1);
        assert!(conversations[0].is_group);
        assert_eq!(
            conversations[0].last_message_ts_unix_ms,
            Some(1_728_307_200_000)
        );

        assert_eq!(
            conversations[0].participants,
            vec!["+15550000002", "+15550000003", "x@example.com"]
        );

        assert_eq!(conversations[1].chat_identifier, "+15551234567");
        assert_eq!(conversations[1].participants, vec!["+15551234567"]);
        assert_eq!(conversations[1].message_count, 2);
        assert!(!conversations[1].is_group);
        assert_eq!(
            conversations[1].last_message_ts_unix_ms,
            Some(1_678_307_200_000)
        );

        assert_eq!(conversations[2].chat_identifier, "empty");
        assert_eq!(conversations[2].message_count, 0);
        assert_eq!(conversations[2].last_message_ts_unix_ms, None);
    }
}
