// SPDX-License-Identifier: GPL-3.0-or-later
//! Reader/writer for the `.amber` archive format described in `SPEC.md`:
//! a ZIP container wrapping a SQLite database (`messages.db`), a
//! `manifest.json`, and content-addressed attachments.
//!
//! Independent of `amber-core`: this crate knows how to build and read a
//! `.amber` archive from data shaped like `SPEC.md`'s tables ([`model`]),
//! regardless of where that data came from.

mod error;
mod export;
mod hash;
mod manifest;
mod model;
mod reader;
mod schema;
mod writer;

pub use error::AmberFormatError;
pub use export::{
    system_clock, utc_clock, write_csv, write_csv_with_progress, write_html,
    write_html_with_progress, write_json, write_json_with_progress, write_text,
    write_text_with_progress, Clock, ExportAttachment, ExportConversation, ExportFormat,
    ExportMessage, ExportReaction, LocalTime, MediaLinks,
};
pub use manifest::{
    Manifest, ManifestConversation, ManifestCounts, ManifestExportedRange, ManifestIntegrity,
    ManifestParticipant, ManifestSource,
};
pub use model::{
    ArchiveInput, AttachmentInput, AttachmentRecord, ConversationInput, DayBucketRecord,
    MessageInput, MessageRecord, ParticipantInput, ParticipantRecord, ReactionInput,
    ReactionRecord, SourceInput,
};
pub use reader::{copy_attachment_from, AmberArchive};
pub use writer::{write_archive, write_archive_with_progress, WriteProgress};
