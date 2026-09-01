# The `.amber` archive format — v1

Amber's native format. Designed to be **durable, inspectable, and recoverable
with standard tools** even if Amber is never updated again. It is not a new binary
format: it is a ZIP container wrapping a SQLite database plus media, both of which
are stable, widely supported, and explicitly endorsed as long-term archival
formats.

## 1. Container

A `.amber` file **is a ZIP archive**. (Same approach as `.docx`, `.epub`, `.apk`.)

```
conversation.amber            (ZIP)
├── manifest.json             UTF-8 JSON, see §2
├── messages.db               SQLite database, see §3
├── attachments/              media, content-addressed by SHA-256
│   └── ab/abcdef…1234.heic   attachments/<first 2 hex>/<full sha256>.<ext>
└── thumbnails/               OPTIONAL generated posters for fast scrolling
    └── abcdef…1234.jpg
```

Rules:

- **Media is stored uncompressed** in the ZIP (method `STORE`). Photos and video
  are already compressed; re-deflating them wastes CPU for ~0 gain. `manifest.json`
  and `messages.db` may use `DEFLATE`.
- **Attachments are content-addressed** by the SHA-256 of their bytes. Identical
  media (e.g. a photo sent twice) is stored once. The DB references attachments by
  `sha256` + relative path; the two-level `<first 2 hex>/` prefix keeps directories
  from growing unbounded.
- Thumbnails are a rebuildable cache; a reader must work if `thumbnails/` is absent.

## 2. `manifest.json`

Self-identifies the file and carries versioning and integrity metadata.

```json
{
  "format": "amber-archive",
  "format_version": "1.0.0",
  "schema_version": 1,
  "created_utc": "2026-09-01T12:00:00Z",
  "generator": "Amber 0.1.0",
  "source": {
    "type": "device-backup",
    "device_name": "Iyaas's iPhone",
    "ios_version": "18.6",
    "exported_range": { "from": "2018-05-01T…Z", "to": "2025-09-01T…Z" }
  },
  "conversation": {
    "chat_identifier": "+32XXXXXXXXX",
    "display_name": "…",
    "is_group": false,
    "participants": [
      { "identifier": "+32XXXXXXXXX", "display_name": "…", "is_me": false },
      { "identifier": "me",            "display_name": "…", "is_me": true }
    ]
  },
  "counts": { "messages": 48213, "attachments": 5120 },
  "integrity": {
    "messages_db_sha256": "…",
    "attachments_sha256_indexed": true
  }
}
```

- `source.type` ∈ `device-backup` | `ios-backup` | `macos-live`.
- `integrity.messages_db_sha256` lets a reader verify the DB wasn't corrupted.

## 3. `messages.db` (SQLite schema v1)

The database self-identifies via SQLite pragmas — its own magic number and schema
version, independent of the manifest:

```sql
PRAGMA application_id = 0x414D4252;  -- ASCII 'AMBR'
PRAGMA user_version   = 1;           -- schema_version; bump on schema change
```

### Tables

```sql
CREATE TABLE participant (
  id            INTEGER PRIMARY KEY,
  identifier    TEXT NOT NULL,          -- phone / email / handle, or 'me'
  display_name  TEXT,
  is_me         INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE message (
  id             INTEGER PRIMARY KEY,
  guid           TEXT UNIQUE,           -- source GUID → idempotent re-import
  participant_id INTEGER REFERENCES participant(id),
  is_from_me     INTEGER NOT NULL DEFAULT 0,
  ts_unix_ms     INTEGER NOT NULL,      -- canonical: ms since Unix epoch, UTC
  service        TEXT,                  -- 'iMessage' | 'SMS'
  text           TEXT,                  -- decoded body (attributedBody resolved)
  reply_to_guid  TEXT,                  -- threaded-reply target, nullable
  is_edited      INTEGER NOT NULL DEFAULT 0,
  is_unsent      INTEGER NOT NULL DEFAULT 0,
  edit_history   TEXT                   -- JSON array of prior texts, nullable
);
CREATE INDEX idx_message_ts ON message(ts_unix_ms);

CREATE TABLE attachment (
  id           INTEGER PRIMARY KEY,
  message_id   INTEGER REFERENCES message(id),
  sha256       TEXT NOT NULL,           -- content address
  rel_path     TEXT NOT NULL,           -- e.g. attachments/ab/abcdef….heic
  mime_type    TEXT,
  filename     TEXT,                    -- original transfer name
  byte_size    INTEGER,
  width        INTEGER,
  height       INTEGER,
  duration_ms  INTEGER,                 -- audio/video
  thumb_path   TEXT                     -- optional poster, nullable
);
CREATE INDEX idx_attachment_message ON attachment(message_id);

CREATE TABLE reaction (                  -- tapbacks
  id                  INTEGER PRIMARY KEY,
  target_message_guid TEXT NOT NULL,
  participant_id      INTEGER REFERENCES participant(id),
  kind                TEXT NOT NULL,     -- love|like|dislike|laugh|emphasize|question|emoji
  emoji               TEXT,              -- for arbitrary-emoji tapbacks
  ts_unix_ms          INTEGER,
  is_removed          INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_reaction_target ON reaction(target_message_guid);
```

### Search (recommended)

```sql
CREATE VIRTUAL TABLE message_fts USING fts5(
  text, content='message', content_rowid='id'
);
```

### Timeline index (cache for the scrubber)

Materialized day buckets so the Photos-style scrubber is O(1) to render and
jump-to-date is instant. Fully rebuildable from `message`, so a reader may
regenerate it if missing.

```sql
CREATE TABLE day_index (
  day           TEXT PRIMARY KEY,       -- 'YYYY-MM-DD' (UTC)
  message_count INTEGER NOT NULL,
  first_msg_id  INTEGER,                -- anchor for jump-to-date
  last_msg_id   INTEGER
);
```

### Time representation

Apple stores message dates as nanoseconds since 2001-01-01 UTC (older DBs: whole
seconds). On import, Amber converts to **Unix milliseconds** as the single
canonical value (`ts_unix_ms`). Store UTC; render in the viewer's local zone.

## 4. Versioning & compatibility

- `format_version` is **semver**. A reader **must reject a higher major** version
  and **may read a higher minor**, ignoring unknown JSON fields.
- `schema_version` (= SQLite `user_version`) is a monotonic integer. Schema
  changes are **forward-only migrations**; a reader that knows version *N* may open
  ≤ *N* and must refuse > *N* rather than guess.
- A valid `.amber` reader validates, in order: it is a ZIP → `manifest.json`
  parses and `format == "amber-archive"` → `application_id == 0x414D4252` →
  versions are in range → (optional) `messages_db_sha256` matches.

## 5. Import / export semantics

- **Export** = build a bundle from Amber's working store. Lossless for everything
  in the schema; unknown source fields Amber chooses not to model are dropped, not
  hidden.
- **Import** = validate (§4) then either open read-only for viewing, or **merge**
  into a working store keyed by `message.guid` (and `attachment.sha256`). Merge is
  **idempotent**: re-importing the same phone or a refreshed backup updates in
  place and adds only what's new — this is what makes incremental device re-syncs
  cheap.

## 6. Explicitly out of scope for v1

Group-chat multi-party rendering nuances beyond participants + per-message sender;
message effects/stickers/app-messages beyond text + standard attachments;
encryption of the `.amber` file itself (the bundle is plaintext — treat it like
any sensitive personal file). These may extend the schema in later minor versions
without breaking v1 readers.
EOF