// SPDX-License-Identifier: GPL-3.0-or-later
//! UI-independent domain model for a parsed conversation.
//!
//! These types are what `amber-format` (M2) will serialize into a `.amber`
//! archive and what the Tauri layer (M3+) will hand to the frontend as JSON.

use serde::{Deserialize, Serialize};

/// One message in a conversation, with its plain text already resolved
/// (from `text` directly, or decoded from `attributedBody` when `text` was
/// empty) and its timestamp already normalized to Unix milliseconds (UTC).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DomainMessage {
    /// Source GUID, stable across re-imports (`message.guid` upstream).
    pub guid: String,
    /// Decoded plain-text body, if any.
    pub text: Option<String>,
    /// Canonical send time: Unix milliseconds since epoch, UTC.
    pub ts_unix_ms: i64,
    /// Sender identifier: `"me"` for outgoing messages, otherwise the
    /// sender's handle identifier (phone number, email, or service handle).
    pub sender: String,
    /// `true` when the database owner sent this message.
    pub is_from_me: bool,
    /// Relative/original file paths of any attachments on this message, in
    /// database join order. Resolving these to actual bytes on disk is an
    /// ingest-time (M5) concern, not a parsing one.
    pub attachment_paths: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_json() {
        let message = DomainMessage {
            guid: "abc-123".to_string(),
            text: Some("hello".to_string()),
            ts_unix_ms: 1_700_000_000_000,
            sender: "me".to_string(),
            is_from_me: true,
            attachment_paths: vec!["attachments/ab/abcdef.heic".to_string()],
        };

        let json = serde_json::to_string(&message).unwrap();
        let round_tripped: DomainMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(message, round_tripped);
    }
}
