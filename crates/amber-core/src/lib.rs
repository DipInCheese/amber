// SPDX-License-Identifier: GPL-3.0-or-later
//! Domain model and iMessage/SMS database parsing for Amber.
//!
//! This crate is UI-independent: it knows how to turn a Messages
//! `chat.db`-shaped SQLite file into [`model::DomainMessage`]s, and nothing
//! else. Where that database file comes from (a live Mac, a device backup,
//! or an iTunes backup) is `amber-ingest`'s job; turning parsed messages into
//! a `.amber` archive is `amber-format`'s job.

pub mod error;
pub mod model;
pub mod parse;
pub mod time;

pub use error::AmberCoreError;
pub use model::DomainMessage;
pub use parse::parse_conversation;
