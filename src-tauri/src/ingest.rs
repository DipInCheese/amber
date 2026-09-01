// SPDX-License-Identifier: GPL-3.0-or-later
//! Commands for M5's plug-and-play ingest flow: check prerequisites, list
//! connected devices, resolve a `Source` down to a chat database + pickable
//! conversations, then parse and write the chosen one to a `.amber`.

use std::collections::HashSet;
use std::path::Path;
use std::sync::Mutex;

use amber_core::{list_conversations, parse_conversation};
use amber_format::{
    write_archive, ArchiveInput, AttachmentInput, ConversationInput, MessageInput,
    ParticipantInput, SourceInput,
};
use amber_ingest::{
    check_prerequisites, list_devices, resolve_backup, resolve_device, resolve_live_mac,
    PrerequisiteReport, ResolvedSource,
};
use serde::Deserialize;
use tauri::{AppHandle, Emitter, State};

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

#[tauri::command]
pub fn check_ingest_prerequisites(app: AppHandle) -> PrerequisiteReportDto {
    let runner = SidecarCommandRunner::new(app);
    let report: PrerequisiteReport = check_prerequisites(&runner, &tools());
    report.into()
}

#[tauri::command]
pub fn list_ingest_devices(app: AppHandle) -> Result<Vec<String>, String> {
    let runner = SidecarCommandRunner::new(app);
    list_devices(&runner, &tools()).map_err(|err| err.to_string())
}

/// Windows only: try to install Apple's USB driver unattended via `winget`.
/// The frontend falls back to opening the Microsoft Store listing itself
/// (`PrerequisiteStatus::remediation_url`) if this errors.
#[tauri::command]
pub fn install_usb_driver(app: AppHandle) -> Result<(), String> {
    let runner = SidecarCommandRunner::new(app);
    amber_ingest::install_usb_driver(&runner)
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
#[tauri::command]
pub fn begin_ingest(
    app: AppHandle,
    source: SourceRequest,
    state: State<IngestState>,
) -> Result<Vec<ConversationSummaryDto>, String> {
    let work_dir = tempfile::tempdir().map_err(|err| err.to_string())?;

    let resolved = match source {
        SourceRequest::LiveMac => {
            resolve_live_mac(work_dir.path()).map_err(|err| err.to_string())?
        }
        SourceRequest::Backup { path, password } => {
            resolve_backup(Path::new(&path), password, work_dir.path())
                .map_err(|err| err.to_string())?
        }
        SourceRequest::Device { udid, password } => {
            let runner = SidecarCommandRunner::new(app.clone());
            let backups_root = work_dir.path().join("device-backup");
            let progress_app = app.clone();
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
#[tauri::command]
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

    // Resolve each attachment's bytes into a temp file for the writer to
    // embed. Best-effort: an attachment amber-ingest can't locate (a
    // partial/corrupt backup, an unusual path) is skipped rather than
    // failing the whole import.
    let attachment_temp_dir = tempfile::tempdir().map_err(|err| err.to_string())?;
    let mut attachments = Vec::new();
    for message in &messages {
        for (index, attachment_path) in message.attachment_paths.iter().enumerate() {
            let Ok(bytes) = pending.resolved.attachment_bytes(attachment_path) else {
                continue;
            };
            let temp_path = attachment_temp_dir
                .path()
                .join(format!("{}-{index}", message.guid));
            std::fs::write(&temp_path, &bytes).map_err(|err| err.to_string())?;

            attachments.push(AttachmentInput {
                message_guid: message.guid.clone(),
                source_path: temp_path,
                mime_type: None,
                filename: Path::new(attachment_path)
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned()),
                width: None,
                height: None,
                duration_ms: None,
            });
        }
    }

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
    write_archive(output_path, &input).map_err(|err| err.to_string())?;
    drop(guard);

    let _ = app.emit("ingest://done", output_path.to_string_lossy().into_owned());
    open_and_store(output_path, &archive_state)
}
