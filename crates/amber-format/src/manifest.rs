// SPDX-License-Identifier: GPL-3.0-or-later
//! `manifest.json`, per `SPEC.md` §2.

use serde::{Deserialize, Serialize};

use crate::error::AmberFormatError;

/// The only major version this reader understands. Per `SPEC.md` §4, a
/// reader must reject a higher major version and may read a higher minor.
pub const SUPPORTED_FORMAT_MAJOR: u64 = 1;

pub const FORMAT_VERSION: &str = "1.0.0";
const FORMAT_TAG: &str = "amber-archive";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    pub format: String,
    pub format_version: String,
    pub schema_version: i64,
    pub created_utc: String,
    pub generator: String,
    pub source: ManifestSource,
    pub conversation: ManifestConversation,
    pub counts: ManifestCounts,
    pub integrity: ManifestIntegrity,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ManifestSource {
    #[serde(rename = "type")]
    pub source_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ios_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exported_range: Option<ManifestExportedRange>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ManifestExportedRange {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ManifestConversation {
    pub chat_identifier: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub is_group: bool,
    pub participants: Vec<ManifestParticipant>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ManifestParticipant {
    pub identifier: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub is_me: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ManifestCounts {
    pub messages: i64,
    pub attachments: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ManifestIntegrity {
    pub messages_db_sha256: String,
    pub attachments_sha256_indexed: bool,
}

impl Manifest {
    /// The `format` and `format_version` checks from the §4 validation
    /// order (steps 2 and 4a). Does not check `application_id` or
    /// `schema_version` - those require the extracted `messages.db`.
    pub fn validate_format(&self) -> Result<(), AmberFormatError> {
        if self.format != FORMAT_TAG {
            return Err(AmberFormatError::WrongFormat(self.format.clone()));
        }

        let major = parse_semver_major(&self.format_version)?;
        if major > SUPPORTED_FORMAT_MAJOR {
            return Err(AmberFormatError::UnsupportedFormatVersion {
                found: self.format_version.clone(),
                supported_major: SUPPORTED_FORMAT_MAJOR,
            });
        }

        Ok(())
    }
}

fn parse_semver_major(version: &str) -> Result<u64, AmberFormatError> {
    version
        .split('.')
        .next()
        .and_then(|major| major.parse::<u64>().ok())
        .ok_or_else(|| {
            AmberFormatError::InvalidSemver(
                version.to_string(),
                "expected a MAJOR.MINOR.PATCH version".to_string(),
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_manifest(format: &str, version: &str) -> Manifest {
        Manifest {
            format: format.to_string(),
            format_version: version.to_string(),
            schema_version: 1,
            created_utc: "2026-09-01T00:00:00Z".to_string(),
            generator: "test".to_string(),
            source: ManifestSource {
                source_type: "macos-live".to_string(),
                device_name: None,
                ios_version: None,
                exported_range: None,
            },
            conversation: ManifestConversation {
                chat_identifier: "+15551234567".to_string(),
                display_name: None,
                is_group: false,
                participants: vec![],
            },
            counts: ManifestCounts {
                messages: 0,
                attachments: 0,
            },
            integrity: ManifestIntegrity {
                messages_db_sha256: "deadbeef".to_string(),
                attachments_sha256_indexed: true,
            },
        }
    }

    #[test]
    fn accepts_matching_major_version() {
        sample_manifest(FORMAT_TAG, "1.4.2")
            .validate_format()
            .unwrap();
    }

    #[test]
    fn rejects_wrong_format_tag() {
        let err = sample_manifest("something-else", "1.0.0")
            .validate_format()
            .unwrap_err();
        assert!(matches!(err, AmberFormatError::WrongFormat(_)));
    }

    #[test]
    fn rejects_newer_major_version() {
        let err = sample_manifest(FORMAT_TAG, "2.0.0")
            .validate_format()
            .unwrap_err();
        assert!(matches!(
            err,
            AmberFormatError::UnsupportedFormatVersion { .. }
        ));
    }
}
