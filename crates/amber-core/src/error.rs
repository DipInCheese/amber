// SPDX-License-Identifier: GPL-3.0-or-later
use imessage_database::error::table::TableError;

/// Errors that can occur while parsing a Messages database.
#[derive(Debug, thiserror::Error)]
pub enum AmberCoreError {
    #[error("no conversation matching {0:?} was found in this database")]
    ConversationNotFound(String),

    #[error("database error: {0}")]
    Table(#[from] TableError),

    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
}
