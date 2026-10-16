// SPDX-License-Identifier: GPL-3.0-or-later
//! Turns phone numbers and email addresses into the names the phone's owner
//! saved for them, using the address book that travels in an iPhone backup
//! (`AddressBook.sqlitedb`). Read-only; nothing is written back.

use std::collections::HashMap;
use std::path::Path;

use rusqlite::{Connection, OpenFlags};

use crate::error::AmberCoreError;

/// `ABMultiValue.property` values in the iOS address book.
const PROPERTY_PHONE: i64 = 3;
const PROPERTY_EMAIL: i64 = 4;

/// How many trailing digits identify a phone number. Enough to tell numbers
/// apart, few enough that "+32 475 12 34 56" and "0475 12 34 56" - the same
/// number written with and without the country code - agree.
const PHONE_KEY_DIGITS: usize = 9;

/// A lookup from message handles (what Messages calls a sender) to contact names.
#[derive(Debug, Clone, Default)]
pub struct Contacts {
    names: HashMap<String, String>,
}

impl Contacts {
    /// No contacts: every lookup misses and callers fall back to the raw handle.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Read every named person's phone numbers and emails out of an
    /// `AddressBook.sqlitedb`.
    pub fn load(address_book: &Path) -> Result<Self, AmberCoreError> {
        let db = Connection::open_with_flags(address_book, OpenFlags::SQLITE_OPEN_READ_ONLY)?;

        let mut statement = db.prepare(
            "SELECT p.First, p.Last, p.Organization, p.Nickname, v.property, v.value
               FROM ABMultiValue v
               JOIN ABPerson p ON p.ROWID = v.record_id
              WHERE v.property IN (3, 4) AND v.value IS NOT NULL
              ORDER BY p.ROWID",
        )?;

        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, Option<String>>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, String>(5)?,
            ))
        })?;

        let mut names = HashMap::new();
        for row in rows {
            let (first, last, organization, nickname, property, value) = row?;
            let Some(name) = display_name(first, last, organization, nickname) else {
                continue;
            };
            let key = match property {
                PROPERTY_PHONE | PROPERTY_EMAIL => handle_key(&value),
                _ => None,
            };
            if let Some(key) = key {
                // The first person who owns a number wins, as in the phone itself.
                names.entry(key).or_insert(name);
            }
        }

        Ok(Self { names })
    }

    pub fn len(&self) -> usize {
        self.names.len()
    }

    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }

    /// The saved name for a phone number or email address, if there is one.
    pub fn name_for(&self, handle: &str) -> Option<&str> {
        self.names.get(&handle_key(handle)?).map(String::as_str)
    }

    /// A label for a conversation: its own name if it has one (a named
    /// group), else the contact's name for a one-to-one chat, else the
    /// people in an unnamed group ("Marie, Jonas & Sofie").
    pub fn conversation_label(
        &self,
        chat_identifier: &str,
        chat_name: Option<&str>,
        is_group: bool,
        participants: &[String],
    ) -> Option<String> {
        if let Some(name) = chat_name.filter(|name| !name.trim().is_empty()) {
            return Some(name.to_string());
        }
        if !is_group {
            return self.name_for(chat_identifier).map(str::to_string);
        }

        let names: Vec<String> = participants
            .iter()
            .map(|handle| match self.name_for(handle) {
                Some(name) => name.split_whitespace().next().unwrap_or(name).to_string(),
                None => handle.clone(),
            })
            .collect();
        match names.as_slice() {
            [] => None,
            [only] => Some(only.clone()),
            [first, second] => Some(format!("{first} & {second}")),
            [first, second, third] => Some(format!("{first}, {second} & {third}")),
            [first, second, rest @ ..] => {
                Some(format!("{first}, {second} & {} others", rest.len()))
            }
        }
    }
}

fn display_name(
    first: Option<String>,
    last: Option<String>,
    organization: Option<String>,
    nickname: Option<String>,
) -> Option<String> {
    let clean = |value: Option<String>| {
        value
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
    };
    let (first, last) = (clean(first), clean(last));
    match (first, last) {
        (Some(first), Some(last)) => Some(format!("{first} {last}")),
        (Some(only), None) | (None, Some(only)) => Some(only),
        (None, None) => clean(organization).or_else(|| clean(nickname)),
    }
}

/// A key two spellings of the same handle share: lower-cased for an email;
/// the last digits for a phone number. `None` for something that is neither
/// (a short code, an empty value).
pub fn handle_key(handle: &str) -> Option<String> {
    let handle = handle.trim();
    if handle.contains('@') {
        return Some(handle.to_lowercase());
    }
    let digits: String = handle.chars().filter(char::is_ascii_digit).collect();
    if digits.len() < 7 {
        return None;
    }
    Some(format!(
        "tel:{}",
        &digits[digits.len() - digits.len().min(PHONE_KEY_DIGITS)..]
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn address_book() -> tempfile::TempPath {
        let file = tempfile::NamedTempFile::new().unwrap();
        let path = file.into_temp_path();
        let db = Connection::open(&path).unwrap();
        db.execute_batch(
            "
            CREATE TABLE ABPerson (ROWID INTEGER PRIMARY KEY, First TEXT, Last TEXT, Organization TEXT, Nickname TEXT);
            CREATE TABLE ABMultiValue (UID INTEGER PRIMARY KEY, record_id INTEGER, property INTEGER, label INTEGER, value TEXT);
            INSERT INTO ABPerson VALUES (1, 'Marie', 'Dubois', NULL, NULL);
            INSERT INTO ABPerson VALUES (2, NULL, NULL, 'Pizza Palace', NULL);
            INSERT INTO ABPerson VALUES (3, 'Jonas', NULL, NULL, NULL);
            INSERT INTO ABMultiValue (record_id, property, value) VALUES
                (1, 3, '0475 12 34 56'),
                (1, 4, 'Marie@Example.com'),
                (2, 3, '+32 2 555 01 23'),
                (3, 3, '+1 (555) 010-9999'),
                (3, 5, 'somewhere');
            ",
        )
        .unwrap();
        path
    }

    #[test]
    fn finds_a_name_whatever_way_the_number_is_written() {
        let contacts = Contacts::load(&address_book()).unwrap();

        assert_eq!(contacts.name_for("+32475123456"), Some("Marie Dubois"));
        assert_eq!(contacts.name_for("0475123456"), Some("Marie Dubois"));
        assert_eq!(contacts.name_for("marie@example.com"), Some("Marie Dubois"));
        assert_eq!(contacts.name_for("+3225550123"), Some("Pizza Palace"));
        assert_eq!(contacts.name_for("+15550109999"), Some("Jonas"));
        assert_eq!(contacts.name_for("+15551112222"), None);
        assert_eq!(contacts.name_for("12345"), None);
    }

    #[test]
    fn labels_conversations() {
        let contacts = Contacts::load(&address_book()).unwrap();
        let handles = |list: &[&str]| list.iter().map(|h| h.to_string()).collect::<Vec<_>>();

        assert_eq!(
            contacts.conversation_label("+32475123456", None, false, &[]),
            Some("Marie Dubois".to_string())
        );
        assert_eq!(
            contacts.conversation_label("chat1", Some("Family"), true, &[]),
            Some("Family".to_string())
        );
        assert_eq!(
            contacts.conversation_label(
                "chat2",
                None,
                true,
                &handles(&["+32475123456", "+15550109999", "+15559998888"])
            ),
            Some("Marie, Jonas & +15559998888".to_string())
        );
        assert_eq!(
            contacts.conversation_label("+15551112222", None, false, &[]),
            None
        );
    }

    #[test]
    fn empty_contacts_know_nobody() {
        assert_eq!(Contacts::empty().name_for("+32475123456"), None);
        assert!(Contacts::empty().is_empty());
    }
}
