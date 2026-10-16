// SPDX-License-Identifier: GPL-3.0-or-later
//! Commands for M5's plug-and-play ingest flow: check prerequisites, list
//! connected devices, resolve a `Source` down to a chat database + pickable
//! conversations, then parse and write the chosen one to a `.amber`.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use amber_core::{list_conversations, parse_conversation, DomainMessage};
use amber_format::{
    write_archive_with_progress, ArchiveInput, AttachmentInput, ConversationInput, MessageInput,
    ParticipantInput, SourceInput, WriteProgress,
};
use amber_ingest::{
    check_prerequisites, list_devices, resolve_backup, resolve_device, resolve_live_mac,
    PrerequisiteReport, ResolvedSource,
};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::commands::{open_and_store, AppState};
use crate::dto::{ConversationSummaryDto, OpenArchiveResult, PrerequisiteReportDto};
use crate::sidecar::SidecarCommandRunner;
use crate::tools::tools;

/// A source that's been resolved to a chat database, waiting for the user
/// to pick which conversation to import. Kept alive (including its temp
/// work directory) between `begin_ingest` and `ingest_conversation`.
struct PendingIngest {
    resolved: ResolvedSource,
    _work_dir: tempfile::TempDir,
}

#[derive(Default)]
pub struct IngestState {
    pending: Mutex<Option<PendingIngest>>,
}

#[tauri::command(async)]
pub fn check_ingest_prerequisites(app: AppHandle) -> PrerequisiteReportDto {
    let runner = SidecarCommandRunner::new(app);
    let report: PrerequisiteReport = check_prerequisites(&runner, &tools());
    report.into()
}

#[tauri::command(async)]
pub fn list_ingest_devices(app: AppHandle) -> Result<Vec<String>, String> {
    let runner = SidecarCommandRunner::new(app);
    list_devices(&runner, &tools()).map_err(|err| err.to_string())
}

/// Where phone backups live between imports. iOS can only back up the whole
/// device, so keeping the result here (instead of a throwaway temp dir) is
/// what lets later imports of other conversations skip the phone entirely
/// and a refresh transfer only what changed. It's Amber's own copy - the
/// phone itself is never written to.
fn device_backup_cache_root(app: &AppHandle, custom: Option<&str>) -> Result<PathBuf, String> {
    // A folder the user picked (the saved backup is as large as the phone's
    // used storage, and the default drive is often a small system drive).
    // Always a subfolder of it, so "Delete saved backup" can never remove
    // anything else they keep there.
    if let Some(dir) = custom.filter(|dir| !dir.trim().is_empty()) {
        return Ok(Path::new(dir).join("Amber phone backup"));
    }
    app.path()
        .app_local_data_dir()
        .map(|dir| dir.join("device-backups"))
        .map_err(|err| err.to_string())
}

fn dir_size(path: &Path) -> u64 {
    let Ok(entries) = std::fs::read_dir(path) else {
        return 0;
    };
    entries
        .flatten()
        .map(|entry| match entry.metadata() {
            Ok(meta) if meta.is_dir() => dir_size(&entry.path()),
            Ok(meta) => meta.len(),
            Err(_) => 0,
        })
        .sum()
}

#[derive(Debug, Serialize)]
pub struct DeviceBackupCacheDto {
    /// A complete backup directory, usable as a `Backup` source path.
    pub path: String,
    pub size_bytes: u64,
}

/// The most recently refreshed complete phone backup Amber is holding, if
/// any. Interrupted/partial backups (no `Manifest.plist` yet) don't count.
#[tauri::command(async)]
pub fn device_backup_cache_info(
    app: AppHandle,
    backup_dir: Option<String>,
) -> Result<Option<DeviceBackupCacheDto>, String> {
    let root = device_backup_cache_root(&app, backup_dir.as_deref())?;
    let Ok(entries) = std::fs::read_dir(&root) else {
        return Ok(None);
    };

    let newest = entries
        .flatten()
        .filter(|entry| entry.path().join("Manifest.plist").is_file())
        .filter_map(|entry| {
            let modified = entry.metadata().ok()?.modified().ok()?;
            Some((modified, entry.path()))
        })
        .max_by_key(|(modified, _)| *modified);

    Ok(newest.map(|(_, path)| DeviceBackupCacheDto {
        size_bytes: dir_size(&path),
        path: path.to_string_lossy().into_owned(),
    }))
}

/// Delete every cached phone backup (and drop any in-progress import that
/// might still be reading from one).
#[tauri::command(async)]
pub fn clear_device_backup_cache(
    app: AppHandle,
    state: State<IngestState>,
    backup_dir: Option<String>,
) -> Result<(), String> {
    *state.pending.lock().unwrap() = None;

    let root = device_backup_cache_root(&app, backup_dir.as_deref())?;
    match std::fs::remove_dir_all(&root) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err.to_string()),
    }
}

/// What the frontend sends to start an import - mirrors [`amber_ingest::Source`]
/// but as a `serde`-friendly, JSON-tagged request (`Source` itself isn't
/// `Deserialize`, since `amber-ingest` has no reason to depend on `serde`
/// for its own internal use).
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SourceRequest {
    Device {
        udid: Option<String>,
        password: Option<String>,
        /// Folder to keep the saved phone backup in (can be tens of GB).
        /// Defaults to Amber's own app-data folder.
        backup_dir: Option<String>,
    },
    Backup {
        path: String,
        password: Option<String>,
    },
    LiveMac,
}

/// Resolve `source` to a chat database and return the conversations found
/// in it, so the frontend can let the user pick one. Stashes the resolved
/// source in [`IngestState`] for the follow-up `ingest_conversation` call.
#[tauri::command(async)]
pub fn begin_ingest(
    app: AppHandle,
    source: SourceRequest,
    state: State<IngestState>,
) -> Result<Vec<ConversationSummaryDto>, String> {
    // Scratch space for the extracted Messages database - small, so the
    // system temp folder is fine. (The big thing, the saved phone backup,
    // lives in its own folder; see device_backup_cache_root.)
    let work_dir = tempfile::tempdir().map_err(|err| err.to_string())?;

    let resolved = match source {
        SourceRequest::LiveMac => {
            resolve_live_mac(work_dir.path()).map_err(|err| err.to_string())?
        }
        SourceRequest::Backup { path, password } => {
            resolve_backup(Path::new(&path), password, work_dir.path())
                .map_err(|err| err.to_string())?
        }
        SourceRequest::Device {
            udid,
            password,
            backup_dir,
        } => {
            let runner = SidecarCommandRunner::new(app.clone());
            let backups_root = device_backup_cache_root(&app, backup_dir.as_deref())?;
            let progress_app = app.clone();
            let log_app = app.clone();
            resolve_device(
                &runner,
                &tools(),
                udid.as_deref(),
                password.as_deref(),
                &backups_root,
                work_dir.path(),
                move |percent| {
                    let _ = progress_app.emit("ingest://backup-progress", percent);
                },
                move |line| {
                    let _ = log_app.emit("ingest://backup-log", line);
                },
            )
            .map_err(|err| err.to_string())?
        }
    };

    let conversations =
        list_conversations(&resolved.chat_db_path).map_err(|err| err.to_string())?;

    *state.pending.lock().unwrap() = Some(PendingIngest {
        resolved,
        _work_dir: work_dir,
    });

    Ok(conversations.into_iter().map(Into::into).collect())
}

/// Parse the conversation `chat_identifier` out of the source resolved by
/// [`begin_ingest`], write it to `output_path` as a `.amber`, then open
/// that archive as the current one (same as [`crate::commands::open_archive`]).
///
/// **Known gap:** `amber-core::parse_conversation` (M1) doesn't yet extract
/// reactions, edit history, unsent status, or reply targets, so archives
/// written through this path currently carry text/timestamp/sender/
/// attachments only. Enriching the M1 parser to close that gap is tracked
/// separately rather than folded into this milestone.
#[tauri::command(async)]
pub fn ingest_conversation(
    app: AppHandle,
    chat_identifier: String,
    output_path: String,
    ingest_state: State<IngestState>,
    archive_state: State<AppState>,
) -> Result<OpenArchiveResult, String> {
    let guard = ingest_state.pending.lock().unwrap();
    let pending = guard
        .as_ref()
        .ok_or("no source has been resolved - call begin_ingest first")?;

    let messages = parse_conversation(&pending.resolved.chat_db_path, &chat_identifier)
        .map_err(|err| err.to_string())?;

    let mut participants = vec![ParticipantInput {
        identifier: "me".to_string(),
        display_name: Some("Me".to_string()),
        is_me: true,
    }];
    let mut seen_senders: HashSet<String> = HashSet::from(["me".to_string()]);
    for message in &messages {
        if seen_senders.insert(message.sender.clone()) {
            participants.push(ParticipantInput {
                identifier: message.sender.clone(),
                display_name: None,
                is_me: false,
            });
        }
    }

    let attachment_temp_dir = tempfile::tempdir().map_err(|err| err.to_string())?;
    let (attachments, missing) =
        collect_attachments(&pending.resolved, &messages, attachment_temp_dir.path());
    let attachments_imported = attachments.len();

    let input = ArchiveInput {
        generator: format!("Amber {}", env!("CARGO_PKG_VERSION")),
        source: SourceInput {
            source_type: "device-backup".to_string(),
            device_name: pending.resolved.device_name.clone(),
            ios_version: None,
            exported_range: None,
        },
        conversation: ConversationInput {
            chat_identifier: chat_identifier.clone(),
            display_name: None,
            is_group: false,
        },
        participants,
        messages: messages
            .iter()
            .map(|message| MessageInput {
                guid: message.guid.clone(),
                sender_identifier: if message.is_from_me {
                    "me".to_string()
                } else {
                    message.sender.clone()
                },
                is_from_me: message.is_from_me,
                ts_unix_ms: message.ts_unix_ms,
                service: None,
                text: message.text.clone(),
                reply_to_guid: None,
                is_edited: false,
                is_unsent: false,
                edit_history: None,
            })
            .collect(),
        attachments,
        reactions: vec![],
    };

    let output_path = Path::new(&output_path);
    let mut throttle = ProgressThrottle::new();
    write_archive_with_progress(output_path, &input, |progress| {
        let (stage, done, total) = match progress {
            WriteProgress::HashingAttachments { done, total } => ("hashing", done, total),
            WriteProgress::BuildingDatabase => ("database", 0, 0),
            WriteProgress::WritingAttachments { done, total } => ("writing", done, total),
        };
        if throttle.should_emit(stage) {
            let _ = app.emit(
                "ingest://write-progress",
                WriteProgressDto { stage, done, total },
            );
        }
    })
    .map_err(|err| err.to_string())?;
    drop(guard);

    let _ = app.emit("ingest://done", output_path.to_string_lossy().into_owned());
    let mut result = open_and_store(output_path, &archive_state)?;
    result.attachments_imported = Some(attachments_imported);
    result.attachments_missing = Some(missing);
    Ok(result)
}

/// Sent as `ingest://write-progress` while the archive is being written.
#[derive(Debug, Clone, Serialize)]
struct WriteProgressDto {
    /// `"hashing"`, `"database"` or `"writing"`.
    stage: &'static str,
    done: usize,
    total: usize,
}

/// Lets a stage's first event and then at most ~10 per second through: a
/// conversation with tens of thousands of small attachments would otherwise
/// flood the webview with events nobody can read.
struct ProgressThrottle {
    last_stage: &'static str,
    last_emit: std::time::Instant,
}

impl ProgressThrottle {
    fn new() -> Self {
        Self {
            last_stage: "",
            last_emit: std::time::Instant::now(),
        }
    }

    fn should_emit(&mut self, stage: &'static str) -> bool {
        let stage_changed = stage != self.last_stage;
        if stage_changed || self.last_emit.elapsed() >= std::time::Duration::from_millis(100) {
            self.last_stage = stage;
            self.last_emit = std::time::Instant::now();
            return true;
        }
        false
    }
}

fn is_plugin_payload(attachment_path: &str) -> bool {
    attachment_path
        .to_ascii_lowercase()
        .ends_with(".pluginpayloadattachment")
}

/// Point the writer at each attachment's bytes. An unencrypted backup or a
/// live Mac already has them as plain files, which are referenced in place
/// (no multi-GB temp copy); only decrypted ones go to scratch files under
/// `scratch_dir`. Returns the attachments plus how many the source didn't
/// actually contain (typically media that lives only in iCloud) - counted
/// and reported rather than failing the import, or, as it once did,
/// vanishing without a word.
fn collect_attachments(
    resolved: &ResolvedSource,
    messages: &[DomainMessage],
    scratch_dir: &Path,
) -> (Vec<AttachmentInput>, usize) {
    let mut attachments = Vec::new();
    let mut missing = 0usize;

    for message in messages {
        for (index, attachment_path) in message.attachment_paths.iter().enumerate() {
            // Rich-link / app-extension payloads: binary plists that back a
            // link preview, not media anyone can view.
            if is_plugin_payload(attachment_path) {
                continue;
            }

            let scratch = scratch_dir.join(format!("{}-{index}", message.guid));
            let Ok(source_path) = resolved.attachment_file(attachment_path, &scratch) else {
                missing += 1;
                continue;
            };

            let filename = Path::new(attachment_path)
                .file_name()
                .map(|name| name.to_string_lossy().into_owned());
            // The viewer picks <img>/<video>/<audio> from this, so it has to
            // be set - `None` would render every attachment as a plain link.
            let mime_type = filename
                .as_deref()
                .map(crate::mime::guess_from_extension)
                .filter(|mime| *mime != "application/octet-stream")
                .map(str::to_string);

            attachments.push(AttachmentInput {
                message_guid: message.guid.clone(),
                source_path,
                mime_type,
                filename,
                width: None,
                height: None,
                duration_ms: None,
            });
        }
    }

    (attachments, missing)
}

#[cfg(test)]
mod tests {
    use super::*;
    use amber_ingest::{AttachmentLocator, IngestError};

    /// Knows exactly one attachment, backed by a real file on disk.
    struct OneFileLocator {
        known_path: &'static str,
        file: PathBuf,
    }

    impl AttachmentLocator for OneFileLocator {
        fn resolve(&self, path: &str) -> Result<Vec<u8>, IngestError> {
            if path == self.known_path {
                Ok(std::fs::read(&self.file)?)
            } else {
                Err(IngestError::AttachmentNotFoundInBackup(path.to_string()))
            }
        }

        fn existing_file(&self, path: &str) -> Option<PathBuf> {
            (path == self.known_path).then(|| self.file.clone())
        }
    }

    fn message(guid: &str, paths: &[&str]) -> DomainMessage {
        DomainMessage {
            guid: guid.to_string(),
            text: None,
            ts_unix_ms: 0,
            sender: "me".to_string(),
            is_from_me: true,
            attachment_paths: paths.iter().map(ToString::to_string).collect(),
        }
    }

    #[test]
    fn recognises_link_preview_payloads_but_not_media() {
        assert!(is_plugin_payload(
            "~/Library/SMS/Attachments/db/11/at_0_X/DD7C.pluginPayloadAttachment"
        ));
        assert!(!is_plugin_payload(
            "~/Library/SMS/Attachments/ab/12/GUID/IMG_0001.HEIC"
        ));
    }

    #[test]
    fn imports_found_media_with_a_mime_type_and_counts_the_missing() {
        let temp = tempfile::tempdir().unwrap();
        let photo = temp.path().join("on-disk-photo");
        std::fs::write(&photo, b"heic bytes").unwrap();

        let known = "~/Library/SMS/Attachments/ab/12/GUID/IMG_0001.HEIC";
        let resolved = ResolvedSource::new(
            temp.path().join("chat.db"),
            None,
            Box::new(OneFileLocator {
                known_path: known,
                file: photo.clone(),
            }),
        );
        let messages = vec![message(
            "m1",
            &[
                known,
                // Referenced but not in the source (e.g. iCloud-only):
                "~/Library/SMS/Attachments/cd/34/GUID/IMG_0002.jpeg",
                // Link-preview payload: neither imported nor "missing".
                "~/Library/SMS/Attachments/ef/56/GUID/x.pluginPayloadAttachment",
            ],
        )];

        let scratch = temp.path().join("scratch");
        std::fs::create_dir_all(&scratch).unwrap();
        let (attachments, missing) = collect_attachments(&resolved, &messages, &scratch);

        assert_eq!(missing, 1);
        assert_eq!(attachments.len(), 1);
        assert_eq!(attachments[0].message_guid, "m1");
        assert_eq!(attachments[0].filename.as_deref(), Some("IMG_0001.HEIC"));
        assert_eq!(attachments[0].mime_type.as_deref(), Some("image/heic"));
        // Referenced in place, not copied to scratch.
        assert_eq!(attachments[0].source_path, photo);
    }
}
