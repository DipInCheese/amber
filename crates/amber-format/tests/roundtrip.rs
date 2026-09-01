// SPDX-License-Identifier: GPL-3.0-or-later
//! End-to-end tests against `amber-format`'s public API: write a `.amber`
//! archive, reopen it, and check everything round-trips per `SPEC.md`.

use std::io::Write as _;

use amber_format::{
    write_archive, AmberArchive, ArchiveInput, AttachmentInput, ConversationInput, MessageInput,
    ParticipantInput, ReactionInput, SourceInput,
};

fn sample_input(attachment_source: std::path::PathBuf) -> ArchiveInput {
    ArchiveInput {
        generator: "amber-test 0.0.0".to_string(),
        source: SourceInput {
            source_type: "macos-live".to_string(),
            device_name: None,
            ios_version: None,
            exported_range: None,
        },
        conversation: ConversationInput {
            chat_identifier: "+15551234567".to_string(),
            display_name: Some("Test Chat".to_string()),
            is_group: false,
        },
        participants: vec![
            ParticipantInput {
                identifier: "+15551234567".to_string(),
                display_name: Some("Alice".to_string()),
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
                guid: "msg-1".to_string(),
                sender_identifier: "+15551234567".to_string(),
                is_from_me: false,
                // 2020-01-01T00:00:00Z
                ts_unix_ms: 1_577_836_800_000,
                service: Some("iMessage".to_string()),
                text: Some("hey!".to_string()),
                reply_to_guid: None,
                is_edited: false,
                is_unsent: false,
                edit_history: None,
            },
            MessageInput {
                guid: "msg-2".to_string(),
                sender_identifier: "me".to_string(),
                is_from_me: true,
                // 2023-06-15T00:00:00Z - a different day than msg-1
                ts_unix_ms: 1_686_787_200_000,
                service: Some("iMessage".to_string()),
                text: Some("hey back, edited".to_string()),
                reply_to_guid: Some("msg-1".to_string()),
                is_edited: true,
                is_unsent: false,
                edit_history: Some(vec!["hey back".to_string()]),
            },
        ],
        attachments: vec![AttachmentInput {
            message_guid: "msg-1".to_string(),
            source_path: attachment_source,
            mime_type: Some("image/jpeg".to_string()),
            filename: Some("photo.jpg".to_string()),
            width: Some(100),
            height: Some(80),
            duration_ms: None,
        }],
        reactions: vec![ReactionInput {
            target_message_guid: "msg-2".to_string(),
            participant_identifier: "+15551234567".to_string(),
            kind: "loved".to_string(),
            emoji: None,
            ts_unix_ms: Some(1_686_787_260_000),
            is_removed: false,
        }],
    }
}

fn write_synthetic_attachment(dir: &tempfile::TempDir, bytes: &[u8]) -> std::path::PathBuf {
    let path = dir.path().join("photo.jpg");
    let mut file = std::fs::File::create(&path).unwrap();
    file.write_all(bytes).unwrap();
    path
}

#[test]
fn round_trips_everything_through_a_written_archive() {
    let work_dir = tempfile::tempdir().unwrap();
    let attachment_bytes = b"not a real jpeg, just test bytes";
    let attachment_path = write_synthetic_attachment(&work_dir, attachment_bytes);

    let input = sample_input(attachment_path);
    let archive_path = work_dir.path().join("conversation.amber");
    write_archive(&archive_path, &input).unwrap();

    let archive = AmberArchive::open(&archive_path).unwrap();

    // Manifest
    let manifest = archive.manifest();
    assert_eq!(manifest.format, "amber-archive");
    assert_eq!(manifest.conversation.chat_identifier, "+15551234567");
    assert_eq!(manifest.counts.messages, 2);
    assert_eq!(manifest.counts.attachments, 1);
    assert!(manifest.integrity.attachments_sha256_indexed);

    // Participants
    let participants = archive.list_participants().unwrap();
    assert_eq!(participants.len(), 2);
    assert!(participants.iter().any(|p| p.identifier == "me" && p.is_me));
    assert!(participants
        .iter()
        .any(|p| p.identifier == "+15551234567" && !p.is_me));

    // Messages, ordered by ts_unix_ms
    let messages = archive.list_messages().unwrap();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].guid.as_deref(), Some("msg-1"));
    assert_eq!(messages[1].guid.as_deref(), Some("msg-2"));
    assert!(messages[1].is_edited);
    assert_eq!(messages[1].edit_history, Some(vec!["hey back".to_string()]));
    assert_eq!(messages[1].reply_to_guid.as_deref(), Some("msg-1"));

    // Attachments: DB row + actual bytes recoverable from the archive
    let attachments = archive.list_attachments().unwrap();
    assert_eq!(attachments.len(), 1);
    let attachment = &attachments[0];
    assert_eq!(attachment.byte_size, Some(attachment_bytes.len() as i64));
    assert!(attachment.rel_path.starts_with("attachments/"));
    assert!(attachment.rel_path.contains(&attachment.sha256));

    let recovered_bytes = archive.read_attachment(&attachment.rel_path).unwrap();
    assert_eq!(recovered_bytes, attachment_bytes);

    // Reactions
    let reactions = archive.list_reactions().unwrap();
    assert_eq!(reactions.len(), 1);
    assert_eq!(reactions[0].target_message_guid, "msg-2");
    assert_eq!(reactions[0].kind, "loved");

    // day_index: msg-1 and msg-2 are ~3.5 years apart, so two buckets.
    let day_buckets = archive.list_day_buckets().unwrap();
    assert_eq!(day_buckets.len(), 2);
    let total: i64 = day_buckets.iter().map(|b| b.message_count).sum();
    assert_eq!(total, 2);
}

#[test]
fn identical_attachment_bytes_are_stored_once() {
    let work_dir = tempfile::tempdir().unwrap();
    let attachment_bytes = b"same bytes, sent twice";
    let attachment_path = write_synthetic_attachment(&work_dir, attachment_bytes);

    let mut input = sample_input(attachment_path.clone());
    // Attach the exact same file to the second message too.
    input.attachments.push(AttachmentInput {
        message_guid: "msg-2".to_string(),
        source_path: attachment_path,
        mime_type: Some("image/jpeg".to_string()),
        filename: Some("photo-again.jpg".to_string()),
        width: None,
        height: None,
        duration_ms: None,
    });

    let archive_path = work_dir.path().join("conversation.amber");
    write_archive(&archive_path, &input).unwrap();

    let archive = AmberArchive::open(&archive_path).unwrap();
    let attachments = archive.list_attachments().unwrap();

    // Two DB rows (one per message)...
    assert_eq!(attachments.len(), 2);
    // ...but the same content-addressed path, and the ZIP entry is only
    // written once (read_attachment would error on a duplicate entry name
    // for most zip readers; succeeding here proves there's exactly one).
    assert_eq!(attachments[0].rel_path, attachments[1].rel_path);
    assert_eq!(attachments[0].sha256, attachments[1].sha256);
    let bytes = archive.read_attachment(&attachments[0].rel_path).unwrap();
    assert_eq!(bytes, attachment_bytes);
}

#[test]
fn tampered_messages_db_fails_the_integrity_check() {
    let work_dir = tempfile::tempdir().unwrap();
    let attachment_path = write_synthetic_attachment(&work_dir, b"x");
    let input = sample_input(attachment_path);

    let archive_path = work_dir.path().join("conversation.amber");
    write_archive(&archive_path, &input).unwrap();

    // Rewrite the ZIP with `messages.db`'s bytes corrupted after the fact,
    // leaving the manifest (and its recorded hash) untouched.
    let tampered_path = work_dir.path().join("tampered.amber");
    {
        let src = std::fs::File::open(&archive_path).unwrap();
        let mut src_zip = zip::ZipArchive::new(src).unwrap();

        let dst = std::fs::File::create(&tampered_path).unwrap();
        let mut dst_zip = zip::ZipWriter::new(dst);
        let options = zip::write::SimpleFileOptions::default();

        for i in 0..src_zip.len() {
            let mut entry = src_zip.by_index(i).unwrap();
            let name = entry.name().to_string();
            let mut bytes = Vec::new();
            std::io::Read::read_to_end(&mut entry, &mut bytes).unwrap();
            drop(entry);

            if name == "messages.db" {
                bytes.extend_from_slice(b"corruption");
            }

            dst_zip.start_file(&name, options).unwrap();
            std::io::Write::write_all(&mut dst_zip, &bytes).unwrap();
        }
        dst_zip.finish().unwrap();
    }

    let result = AmberArchive::open(&tampered_path);
    assert!(matches!(
        result,
        Err(amber_format::AmberFormatError::IntegrityMismatch { .. })
    ));
}
