// SPDX-License-Identifier: GPL-3.0-or-later
//! Reader/writer for the `.amber` archive format described in `SPEC.md`:
//! a ZIP container wrapping a SQLite database (`messages.db`), a
//! `manifest.json`, and content-addressed attachments.
//!
//! Independent of `amber-core`: this crate knows how to build and read a
//! `.amber` archive from data shaped like `SPEC.md`'s tables ([`model`]),
//! regardless of where that data came from.

mod error;
mod hash;
mod manifest;
mod model;
mod reader;
mod schema;
mod writer;

pub use error::AmberFormatError;
pub use manifest::{
    Manifest, ManifestConversation, ManifestCounts, ManifestExportedRange, ManifestIntegrity,
    ManifestParticipant, ManifestSource,
};
pub use model::{
    ArchiveInput, AttachmentInput, AttachmentRecord, ConversationInput, DayBucketRecord,
    MessageInput, MessageRecord, ParticipantInput, ParticipantRecord, ReactionInput,
    ReactionRecord, SourceInput,
};
pub use reader::AmberArchive;
pub use writer::write_archive;
