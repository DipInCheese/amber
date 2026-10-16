// SPDX-License-Identifier: GPL-3.0-or-later
//! Saving a single attachment and exporting the open conversation to
//! other formats. Both read from a *copy of the archive's path*, never
//! while holding the viewer's archive lock, so the conversation stays
//! scrollable (and its photos keep loading) during a long export.

use std::collections::{BTreeSet, HashMap};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use amber_format::{
    copy_attachment_from, system_clock, write_csv, write_html, write_json, write_text,
    AmberArchive, ExportConversation, ExportFormat, MediaLinks,
};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

use crate::commands::AppState;
use crate::heic;

fn open_archive_path(state: &AppState) -> Result<PathBuf, String> {
    let guard = state.archive();
    let archive = guard.as_ref().ok_or("no archive is open")?;
    Ok(archive.zip_path().to_path_buf())
}

/// Write one attachment, exactly as stored (an iPhone photo stays HEIC, a
/// video stays a video), to `dest_path` - a path the user picked in a save
/// dialog.
#[tauri::command(async)]
pub fn save_attachment(
    rel_path: String,
    dest_path: String,
    state: State<AppState>,
) -> Result<(), String> {
    let zip_path = open_archive_path(&state)?;
    let dest = Path::new(&dest_path);

    let result = (|| -> Result<(), String> {
        let mut file = BufWriter::new(File::create(dest).map_err(|err| err.to_string())?);
        copy_attachment_from(&zip_path, &rel_path, &mut file).map_err(|err| err.to_string())?;
        file.flush().map_err(|err| err.to_string())
    })();

    if result.is_err() {
        // Don't leave a truncated file behind that looks like a real photo.
        let _ = std::fs::remove_file(dest);
    }
    result
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormatDto {
    Text,
    Csv,
    Json,
    Html,
}

impl From<ExportFormatDto> for ExportFormat {
    fn from(format: ExportFormatDto) -> Self {
        match format {
            ExportFormatDto::Text => ExportFormat::Text,
            ExportFormatDto::Csv => ExportFormat::Csv,
            ExportFormatDto::Json => ExportFormat::Json,
            ExportFormatDto::Html => ExportFormat::Html,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ExportSummary {
    pub path: String,
    pub messages: usize,
    /// Media files copied next to an HTML export (0 for the other formats).
    pub media_files: usize,
    /// Media the archive listed but couldn't be read back.
    pub media_failed: usize,
}

#[derive(Debug, Clone, Serialize)]
struct ExportProgressDto {
    done: usize,
    total: usize,
}

/// Export the open conversation to `output_path` in `format`. For HTML,
/// `include_media` also copies every photo/video/file into a folder beside
/// the page (`<name>_files/`) and links to them.
#[tauri::command(async)]
pub fn export_conversation(
    app: AppHandle,
    state: State<AppState>,
    format: ExportFormatDto,
    include_media: bool,
    output_path: String,
) -> Result<ExportSummary, String> {
    let zip_path = open_archive_path(&state)?;
    let archive = AmberArchive::open(&zip_path).map_err(|err| err.to_string())?;
    let conversation = ExportConversation::from_archive(&archive).map_err(|err| err.to_string())?;

    let output = Path::new(&output_path);
    let mut media_files = 0;
    let mut media_failed = 0;

    let mut writer = BufWriter::new(File::create(output).map_err(|err| err.to_string())?);
    let written = match ExportFormat::from(format) {
        ExportFormat::Text => write_text(&conversation, &system_clock, &mut writer),
        ExportFormat::Csv => write_csv(&conversation, &system_clock, &mut writer),
        ExportFormat::Json => write_json(&conversation, &mut writer),
        ExportFormat::Html => {
            let links = if include_media {
                let copied = copy_media(&zip_path, &conversation, output, &mut |done, total| {
                    let _ = app.emit("export://progress", ExportProgressDto { done, total });
                })?;
                media_files = copied.links.len();
                media_failed = copied.failed;
                copied.links
            } else {
                HashMap::new()
            };
            write_html(
                &conversation,
                &system_clock,
                &|attachment| links.get(&attachment.rel_path).cloned(),
                &mut writer,
            )
        }
    };
    written
        .and_then(|()| writer.flush())
        .map_err(|err| err.to_string())?;

    Ok(ExportSummary {
        path: output_path,
        messages: conversation.messages.len(),
        media_files,
        media_failed,
    })
}

struct CopiedMedia {
    links: HashMap<String, MediaLinks>,
    failed: usize,
}

/// Copy each distinct attachment into `<output stem>_files/` and return
/// where each one ended up. A HEIC photo also gets a JPEG beside it when
/// this machine can make one, because browsers can't show HEIC.
fn copy_media(
    zip_path: &Path,
    conversation: &ExportConversation,
    output: &Path,
    on_progress: &mut dyn FnMut(usize, usize),
) -> Result<CopiedMedia, String> {
    let stem = output
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "export".to_string());
    let folder_name = format!("{stem}_files");
    let folder = output.with_file_name(&folder_name);
    std::fs::create_dir_all(&folder).map_err(|err| err.to_string())?;
    let url_folder = url_path_segment(&folder_name);

    let distinct: BTreeSet<&str> = conversation
        .messages
        .iter()
        .flat_map(|m| m.attachments.iter().map(|a| a.rel_path.as_str()))
        .collect();
    let total = distinct.len();

    let mut links = HashMap::new();
    let mut failed = 0;
    let mut last_emit = Instant::now() - Duration::from_secs(1);

    for (done, rel_path) in distinct.into_iter().enumerate() {
        if last_emit.elapsed() >= Duration::from_millis(100) {
            on_progress(done, total);
            last_emit = Instant::now();
        }

        let file_name = rel_path.rsplit('/').next().unwrap_or(rel_path);
        let destination = folder.join(file_name);
        let copied = File::create(&destination)
            .map_err(|err| err.to_string())
            .and_then(|file| {
                let mut file = BufWriter::new(file);
                copy_attachment_from(zip_path, rel_path, &mut file)
                    .map_err(|err| err.to_string())?;
                file.flush().map_err(|err| err.to_string())
            });
        if copied.is_err() {
            let _ = std::fs::remove_file(&destination);
            failed += 1;
            continue;
        }

        let preview = if heic::is_heic(rel_path) {
            make_jpeg_preview(&destination)
                .map(|name| format!("{url_folder}/{}", url_path_segment(&name)))
        } else {
            None
        };
        links.insert(
            rel_path.to_string(),
            MediaLinks {
                original: format!("{url_folder}/{}", url_path_segment(file_name)),
                preview,
            },
        );
    }

    on_progress(total, total);
    Ok(CopiedMedia { links, failed })
}

/// Writes `<name>.jpg` next to a copied HEIC, returning its file name.
fn make_jpeg_preview(heic_file: &Path) -> Option<String> {
    let bytes = std::fs::read(heic_file).ok()?;
    let jpeg = heic::to_jpeg(&bytes)?;
    let preview = heic_file.with_extension("jpg");
    std::fs::write(&preview, jpeg).ok()?;
    preview
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
}

/// Percent-encodes one path segment for use in a relative URL.
fn url_path_segment(segment: &str) -> String {
    let mut encoded = String::with_capacity(segment.len());
    for byte in segment.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            other => encoded.push_str(&format!("%{other:02X}")),
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;
    use amber_format::{
        write_archive, ArchiveInput, AttachmentInput, ConversationInput, MessageInput,
        ParticipantInput, SourceInput,
    };

    fn archive_with(dir: &Path, attachments: &[(&str, &str, &[u8])]) -> PathBuf {
        let mut inputs = Vec::new();
        for (index, (filename, mime, bytes)) in attachments.iter().enumerate() {
            let source = dir.join(format!("source-{index}"));
            std::fs::write(&source, bytes).unwrap();
            inputs.push(AttachmentInput {
                message_guid: "m1".to_string(),
                source_path: source,
                mime_type: Some((*mime).to_string()),
                filename: Some((*filename).to_string()),
                width: None,
                height: None,
                duration_ms: None,
            });
        }
        let input = ArchiveInput {
            generator: "test".to_string(),
            source: SourceInput {
                source_type: "device-backup".to_string(),
                device_name: None,
                ios_version: None,
                exported_range: None,
            },
            conversation: ConversationInput {
                chat_identifier: "+15551234567".to_string(),
                display_name: Some("Alice".to_string()),
                is_group: false,
            },
            participants: vec![ParticipantInput {
                identifier: "me".to_string(),
                display_name: Some("Me".to_string()),
                is_me: true,
            }],
            messages: vec![MessageInput {
                guid: "m1".to_string(),
                sender_identifier: "me".to_string(),
                is_from_me: true,
                ts_unix_ms: 1_709_647_331_000,
                service: None,
                text: Some("look".to_string()),
                reply_to_guid: None,
                is_edited: false,
                is_unsent: false,
                edit_history: None,
            }],
            attachments: inputs,
            reactions: vec![],
        };
        let path = dir.join("chat.amber");
        write_archive(&path, &input).unwrap();
        path
    }

    #[test]
    fn copies_each_distinct_attachment_next_to_the_page_and_links_it() {
        let dir = tempfile::tempdir().unwrap();
        let zip_path = archive_with(
            dir.path(),
            &[
                ("photo.jpeg", "image/jpeg", b"jpeg bytes"),
                // Not a decodable HEIC, so no JPEG preview can be made.
                ("IMG_1.HEIC", "image/heic", b"not really heic"),
            ],
        );
        let archive = AmberArchive::open(&zip_path).unwrap();
        let conversation = ExportConversation::from_archive(&archive).unwrap();

        let output = dir.path().join("My chat.html");
        let mut progress = Vec::new();
        let copied = copy_media(&zip_path, &conversation, &output, &mut |done, total| {
            progress.push((done, total));
        })
        .unwrap();

        assert_eq!(copied.failed, 0);
        assert_eq!(copied.links.len(), 2);
        // Ends on "all done", and every link is a relative, URL-safe path.
        assert_eq!(progress.last(), Some(&(2, 2)));
        for links in copied.links.values() {
            assert!(
                links.original.starts_with("My%20chat_files/"),
                "{}",
                links.original
            );
        }
        let folder = dir.path().join("My chat_files");
        let jpeg_rel = &conversation.messages[0].attachments[0].rel_path;
        let file_name = jpeg_rel.rsplit('/').next().unwrap();
        assert_eq!(
            std::fs::read(folder.join(file_name)).unwrap(),
            b"jpeg bytes"
        );
    }

    #[test]
    fn an_attachment_that_cannot_be_read_back_is_counted_not_fatal() {
        let dir = tempfile::tempdir().unwrap();
        let zip_path = archive_with(dir.path(), &[("photo.jpeg", "image/jpeg", b"jpeg bytes")]);
        let archive = AmberArchive::open(&zip_path).unwrap();
        let mut conversation = ExportConversation::from_archive(&archive).unwrap();
        // Point the export at an entry the archive doesn't have.
        conversation.messages[0].attachments[0].rel_path =
            "attachments/zz/missing.jpeg".to_string();

        let output = dir.path().join("chat.html");
        let copied = copy_media(&zip_path, &conversation, &output, &mut |_, _| {}).unwrap();

        assert_eq!(copied.failed, 1);
        assert!(copied.links.is_empty());
        // No empty stub left behind that looks like a real file.
        assert!(!dir.path().join("chat_files/missing.jpeg").exists());
    }

    use super::url_path_segment;

    #[test]
    fn url_segments_keep_safe_characters_and_encode_the_rest() {
        assert_eq!(url_path_segment("abc-123_x.y~z"), "abc-123_x.y~z");
        assert_eq!(
            url_path_segment("my chat (1)_files"),
            "my%20chat%20%281%29_files"
        );
        assert_eq!(url_path_segment("é"), "%C3%A9");
    }
}
