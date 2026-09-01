// SPDX-License-Identifier: GPL-3.0-or-later
//! `messages.db` SQLite schema, verbatim from `SPEC.md` §3.

/// `'AMBR'` in ASCII, as the SQLite `application_id` pragma.
pub const APPLICATION_ID: i64 = 0x414D_4252;

/// Current `messages.db` schema version (SQLite `user_version`).
pub const SCHEMA_VERSION: i64 = 1;

pub const SCHEMA_SQL: &str = "
CREATE TABLE participant (
  id            INTEGER PRIMARY KEY,
  identifier    TEXT NOT NULL,
  display_name  TEXT,
  is_me         INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE message (
  id             INTEGER PRIMARY KEY,
  guid           TEXT UNIQUE,
  participant_id INTEGER REFERENCES participant(id),
  is_from_me     INTEGER NOT NULL DEFAULT 0,
  ts_unix_ms     INTEGER NOT NULL,
  service        TEXT,
  text           TEXT,
  reply_to_guid  TEXT,
  is_edited      INTEGER NOT NULL DEFAULT 0,
  is_unsent      INTEGER NOT NULL DEFAULT 0,
  edit_history   TEXT
);
CREATE INDEX idx_message_ts ON message(ts_unix_ms);

CREATE TABLE attachment (
  id           INTEGER PRIMARY KEY,
  message_id   INTEGER REFERENCES message(id),
  sha256       TEXT NOT NULL,
  rel_path     TEXT NOT NULL,
  mime_type    TEXT,
  filename     TEXT,
  byte_size    INTEGER,
  width        INTEGER,
  height       INTEGER,
  duration_ms  INTEGER,
  thumb_path   TEXT
);
CREATE INDEX idx_attachment_message ON attachment(message_id);

CREATE TABLE reaction (
  id                  INTEGER PRIMARY KEY,
  target_message_guid TEXT NOT NULL,
  participant_id      INTEGER REFERENCES participant(id),
  kind                TEXT NOT NULL,
  emoji               TEXT,
  ts_unix_ms          INTEGER,
  is_removed          INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_reaction_target ON reaction(target_message_guid);

CREATE VIRTUAL TABLE message_fts USING fts5(
  text, content='message', content_rowid='id'
);

CREATE TABLE day_index (
  day           TEXT PRIMARY KEY,
  message_count INTEGER NOT NULL,
  first_msg_id  INTEGER,
  last_msg_id   INTEGER
);
";
