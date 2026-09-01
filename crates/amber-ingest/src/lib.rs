// SPDX-License-Identifier: GPL-3.0-or-later
//! Resolves one of the three `Source` variants (device over USB, an existing
//! iOS/iTunes backup, or a live Mac's `chat.db`) down to "a Messages SQLite
//! database + attachments in a temp directory" that `amber-core::parse` can
//! read.
//!
//! Stubbed for M1; implemented in M5.
