// SPDX-License-Identifier: GPL-3.0-or-later
//! Builds a small, realistic `.amber` sample for manual testing of the
//! viewer (M3+). Not part of the test suite - run explicitly:
//!
//! ```sh
//! cargo run --example build_sample -p amber-format -- /path/to/sample.amber
//! ```

use std::path::PathBuf;

use amber_format::{
    write_archive, ArchiveInput, AttachmentInput, ConversationInput, MessageInput,
    ParticipantInput, ReactionInput, SourceInput,
};

fn main() {
    let output_path = std::env::args()
        .nth(1)
        .expect("usage: build_sample <output-path.amber>");

    let attachment_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/sample_assets/pixel.png");

    let input = ArchiveInput {
        generator: "amber-format sample generator".to_string(),
        source: SourceInput {
            source_type: "macos-live".to_string(),
            device_name: None,
            ios_version: None,
            exported_range: None,
        },
        conversation: ConversationInput {
            chat_identifier: "+15551234567".to_string(),
            display_name: Some("Sam".to_string()),
            is_group: false,
        },
        participants: vec![
            ParticipantInput {
                identifier: "+15551234567".to_string(),
                display_name: Some("Sam".to_string()),
                is_me: false,
            },
            ParticipantInput {
                identifier: "me".to_string(),
                display_name: Some("Me".to_string()),
                is_me: true,
            },
        ],
        messages: vec![
            MessageInput {
                guid: "m1".to_string(),
                sender_identifier: "+15551234567".to_string(),
                is_from_me: false,
                ts_unix_ms: 1_704_067_200_000, // 2024-01-01
                service: Some("iMessage".to_string()),
                text: Some("Happy new year! 🎉".to_string()),
                reply_to_guid: None,
                is_edited: false,
                is_unsent: false,
                edit_history: None,
            },
            MessageInput {
                guid: "m2".to_string(),
                sender_identifier: "me".to_string(),
                is_from_me: true,
                ts_unix_ms: 1_704_067_260_000,
                service: Some("iMessage".to_string()),
                text: Some("You too! Been a while, this year's gonna be a good one.".to_string()),
                reply_to_guid: Some("m1".to_string()),
                is_edited: false,
                is_unsent: false,
                edit_history: None,
            },
            MessageInput {
                guid: "m3".to_string(),
                sender_identifier: "+15551234567".to_string(),
                is_from_me: false,
                ts_unix_ms: 1_704_067_320_000,
                service: Some("iMessage".to_string()),
                text: None,
                reply_to_guid: None,
                is_edited: false,
                is_unsent: false,
                edit_history: None,
            },
            MessageInput {
                guid: "m4".to_string(),
                sender_identifier: "me".to_string(),
                is_from_me: true,
                ts_unix_ms: 1_704_067_380_000,
                service: Some("iMessage".to_string()),
                text: Some("nice pic (fixed the typo)".to_string()),
                reply_to_guid: None,
                is_edited: true,
                is_unsent: false,
                edit_history: Some(vec!["nice pic (fxied the typo)".to_string()]),
            },
            // A different day, to exercise the date separator + scrubber.
            MessageInput {
                guid: "m5".to_string(),
                sender_identifier: "+15551234567".to_string(),
                is_from_me: false,
                ts_unix_ms: 1_706_745_600_000, // 2024-02-01
                service: Some("iMessage".to_string()),
                text: Some("miss you, happy february".to_string()),
                reply_to_guid: None,
                is_edited: false,
                is_unsent: false,
                edit_history: None,
            },
            MessageInput {
                guid: "m6".to_string(),
                sender_identifier: "me".to_string(),
                is_from_me: true,
                ts_unix_ms: 1_706_745_660_000,
                service: Some("iMessage".to_string()),
                text: Some("this one got unsent".to_string()),
                reply_to_guid: None,
                is_edited: false,
                is_unsent: true,
                edit_history: None,
            },
        ],
        attachments: vec![AttachmentInput {
            message_guid: "m3".to_string(),
            source_path: attachment_path,
            mime_type: Some("image/png".to_string()),
            filename: Some("pixel.png".to_string()),
            width: Some(1),
            height: Some(1),
            duration_ms: None,
        }],
        reactions: vec![ReactionInput {
            target_message_guid: "m2".to_string(),
            participant_identifier: "+15551234567".to_string(),
            kind: "love".to_string(),
            emoji: None,
            ts_unix_ms: Some(1_704_067_290_000),
            is_removed: false,
        }],
    };

    write_archive(std::path::Path::new(&output_path), &input).expect("failed to write archive");
    println!("wrote {output_path}");
}
