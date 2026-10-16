// SPDX-License-Identifier: GPL-3.0-or-later
//! Turns an open `.amber` archive into other, more widely readable formats:
//! plain text, CSV, JSON and a self-contained HTML page. The archive itself
//! stays the durable format (`SPEC.md`); these are one-way conveniences for
//! reading, searching or handing a conversation to something else.
//!
//! Everything here is pure formatting over [`ExportConversation`] - no
//! files, no media bytes - so it can be tested directly. Copying media next
//! to an HTML export is the caller's job (see [`MediaLinks`]).

use std::collections::HashMap;
use std::io::Write;

use chrono::{DateTime, Local, SecondsFormat, TimeZone, Utc};
use serde::Serialize;

use crate::error::AmberFormatError;
use crate::reader::AmberArchive;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Text,
    Csv,
    Json,
    Html,
}

impl ExportFormat {
    /// File extension, without the dot.
    #[must_use]
    pub fn extension(self) -> &'static str {
        match self {
            ExportFormat::Text => "txt",
            ExportFormat::Csv => "csv",
            ExportFormat::Json => "json",
            ExportFormat::Html => "html",
        }
    }
}

/// A moment shown in the reader's local time (a plain `ts_unix_ms` is UTC).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalTime {
    /// `2024-03-05`
    pub date: String,
    /// `14:02:11`
    pub time: String,
}

/// Maps a UTC timestamp to the time shown to the reader. The real exporter
/// uses the machine's time zone ([`system_clock`]); tests use [`utc_clock`]
/// so they don't depend on where they run.
pub type Clock<'a> = &'a dyn Fn(i64) -> LocalTime;

fn local_time<Tz: TimeZone>(ts_unix_ms: i64, tz: &Tz) -> LocalTime
where
    Tz::Offset: std::fmt::Display,
{
    match tz.timestamp_millis_opt(ts_unix_ms).single() {
        Some(moment) => LocalTime {
            date: moment.format("%Y-%m-%d").to_string(),
            time: moment.format("%H:%M:%S").to_string(),
        },
        None => LocalTime {
            date: "unknown".to_string(),
            time: String::new(),
        },
    }
}

/// The machine's time zone, daylight saving included - what the viewer shows.
#[must_use]
pub fn system_clock(ts_unix_ms: i64) -> LocalTime {
    local_time(ts_unix_ms, &Local)
}

#[must_use]
pub fn utc_clock(ts_unix_ms: i64) -> LocalTime {
    local_time(ts_unix_ms, &Utc)
}

fn iso_utc(ts_unix_ms: i64) -> String {
    DateTime::<Utc>::from_timestamp_millis(ts_unix_ms)
        .map(|moment| moment.to_rfc3339_opts(SecondsFormat::Millis, true))
        .unwrap_or_default()
}

#[derive(Debug, Clone, Serialize)]
pub struct ExportAttachment {
    pub filename: String,
    pub mime_type: Option<String>,
    pub byte_size: Option<i64>,
    pub sha256: String,
    /// Path inside the `.amber` archive.
    #[serde(rename = "archive_path")]
    pub rel_path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExportReaction {
    /// Who reacted (display name or identifier), when known.
    pub from: Option<String>,
    pub kind: String,
    pub emoji: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExportMessage {
    pub guid: Option<String>,
    pub timestamp_unix_ms: i64,
    pub timestamp_utc: String,
    /// Display name or identifier; `Me` for the archive owner.
    pub sender: String,
    pub is_from_me: bool,
    pub text: Option<String>,
    pub reply_to_guid: Option<String>,
    pub edited: bool,
    pub unsent: bool,
    pub edit_history: Option<Vec<String>>,
    pub attachments: Vec<ExportAttachment>,
    pub reactions: Vec<ExportReaction>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExportConversation {
    pub title: String,
    pub chat_identifier: String,
    pub is_group: bool,
    pub messages: Vec<ExportMessage>,
}

impl ExportConversation {
    /// Read everything an export needs out of `archive`.
    pub fn from_archive(archive: &AmberArchive) -> Result<Self, AmberFormatError> {
        let manifest = archive.manifest();

        let participants: HashMap<i64, String> = archive
            .list_participants()?
            .into_iter()
            .map(|p| {
                let name = if p.is_me {
                    "Me".to_string()
                } else {
                    p.display_name.unwrap_or(p.identifier)
                };
                (p.id, name)
            })
            .collect();

        let mut attachments: HashMap<i64, Vec<ExportAttachment>> = HashMap::new();
        for attachment in archive.list_attachments()? {
            let filename = attachment.filename.clone().unwrap_or_else(|| {
                attachment
                    .rel_path
                    .rsplit('/')
                    .next()
                    .unwrap_or("attachment")
                    .to_string()
            });
            attachments
                .entry(attachment.message_id)
                .or_default()
                .push(ExportAttachment {
                    filename,
                    mime_type: attachment.mime_type,
                    byte_size: attachment.byte_size,
                    sha256: attachment.sha256,
                    rel_path: attachment.rel_path,
                });
        }

        let mut reactions: HashMap<String, Vec<ExportReaction>> = HashMap::new();
        for reaction in archive.list_reactions()? {
            if reaction.is_removed {
                continue;
            }
            reactions
                .entry(reaction.target_message_guid)
                .or_default()
                .push(ExportReaction {
                    from: reaction
                        .participant_id
                        .and_then(|id| participants.get(&id).cloned()),
                    kind: reaction.kind,
                    emoji: reaction.emoji,
                });
        }

        let messages = archive
            .list_messages()?
            .into_iter()
            .map(|message| ExportMessage {
                sender: message
                    .participant_id
                    .and_then(|id| participants.get(&id).cloned())
                    .unwrap_or_else(|| {
                        if message.is_from_me {
                            "Me".to_string()
                        } else {
                            "Unknown".to_string()
                        }
                    }),
                attachments: attachments.remove(&message.id).unwrap_or_default(),
                reactions: message
                    .guid
                    .as_deref()
                    .and_then(|guid| reactions.remove(guid))
                    .unwrap_or_default(),
                guid: message.guid,
                timestamp_unix_ms: message.ts_unix_ms,
                timestamp_utc: iso_utc(message.ts_unix_ms),
                is_from_me: message.is_from_me,
                text: message.text,
                reply_to_guid: message.reply_to_guid,
                edited: message.is_edited,
                unsent: message.is_unsent,
                edit_history: message.edit_history,
            })
            .collect();

        Ok(Self {
            title: manifest
                .conversation
                .display_name
                .clone()
                .unwrap_or_else(|| manifest.conversation.chat_identifier.clone()),
            chat_identifier: manifest.conversation.chat_identifier.clone(),
            is_group: manifest.conversation.is_group,
            messages,
        })
    }
}

// --- Plain text -------------------------------------------------------

pub fn write_text(
    conversation: &ExportConversation,
    clock: Clock<'_>,
    out: &mut dyn Write,
) -> std::io::Result<()> {
    write_text_with_progress(conversation, clock, out, &mut |_| {})
}

/// Like [`write_text`], calling `on_message(n)` as it reaches the n-th message.
pub fn write_text_with_progress(
    conversation: &ExportConversation,
    clock: Clock<'_>,
    out: &mut dyn Write,
    on_message: &mut dyn FnMut(usize),
) -> std::io::Result<()> {
    writeln!(out, "{}", conversation.title)?;
    writeln!(
        out,
        "{} messages - exported from an Amber archive (times in your local time zone)",
        conversation.messages.len()
    )?;

    let mut current_day = String::new();
    for (index, message) in conversation.messages.iter().enumerate() {
        on_message(index + 1);
        let moment = clock(message.timestamp_unix_ms);
        if moment.date != current_day {
            writeln!(out, "\n=== {} ===", moment.date)?;
            current_day = moment.date.clone();
        }

        let time = moment.time.get(..5).unwrap_or(&moment.time);
        if message.unsent {
            writeln!(out, "[{time}] {}: (unsent message)", message.sender)?;
        } else {
            let text = message.text.as_deref().unwrap_or("");
            // Continuation lines of a multi-line message are indented so
            // each message still starts at the left margin.
            let mut lines = text.lines();
            writeln!(
                out,
                "[{time}] {}: {}",
                message.sender,
                lines.next().unwrap_or("")
            )?;
            for line in lines {
                writeln!(out, "    {line}")?;
            }
            if message.edited {
                writeln!(out, "    (edited)")?;
            }
        }
        for attachment in &message.attachments {
            writeln!(out, "    [attachment: {}]", attachment.filename)?;
        }
        for reaction in &message.reactions {
            let what = reaction.emoji.as_deref().unwrap_or(&reaction.kind);
            match &reaction.from {
                Some(from) => writeln!(out, "    [reaction: {what} from {from}]")?,
                None => writeln!(out, "    [reaction: {what}]")?,
            }
        }
    }
    Ok(())
}

// --- CSV --------------------------------------------------------------

fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

pub fn write_csv(
    conversation: &ExportConversation,
    clock: Clock<'_>,
    out: &mut dyn Write,
) -> std::io::Result<()> {
    write_csv_with_progress(conversation, clock, out, &mut |_| {})
}

/// Like [`write_csv`], calling `on_message(n)` as it reaches the n-th message.
pub fn write_csv_with_progress(
    conversation: &ExportConversation,
    clock: Clock<'_>,
    out: &mut dyn Write,
    on_message: &mut dyn FnMut(usize),
) -> std::io::Result<()> {
    // A byte-order mark lets Excel open the file as UTF-8 instead of
    // mangling emoji and accents.
    out.write_all(b"\xEF\xBB\xBF")?;
    writeln!(
        out,
        "timestamp_utc,date,time,sender,direction,text,attachments,edited,unsent,reactions"
    )?;
    for (index, message) in conversation.messages.iter().enumerate() {
        on_message(index + 1);
        let moment = clock(message.timestamp_unix_ms);
        let attachments = message
            .attachments
            .iter()
            .map(|a| a.filename.as_str())
            .collect::<Vec<_>>()
            .join("; ");
        let reactions = message
            .reactions
            .iter()
            .map(|r| r.emoji.as_deref().unwrap_or(&r.kind).to_string())
            .collect::<Vec<_>>()
            .join(" ");
        let fields = [
            message.timestamp_utc.clone(),
            moment.date,
            moment.time,
            message.sender.clone(),
            if message.is_from_me {
                "sent"
            } else {
                "received"
            }
            .to_string(),
            message.text.clone().unwrap_or_default(),
            attachments,
            message.edited.to_string(),
            message.unsent.to_string(),
            reactions,
        ];
        let line: Vec<String> = fields.iter().map(|f| csv_field(f)).collect();
        writeln!(out, "{}", line.join(","))?;
    }
    Ok(())
}

// --- JSON -------------------------------------------------------------

#[derive(Serialize)]
struct JsonExport<'a> {
    format: &'static str,
    conversation: JsonConversation<'a>,
    messages: CountedMessages<'a>,
}

/// The message list, reporting each one as it is written.
struct CountedMessages<'a> {
    messages: &'a [ExportMessage],
    on_message: std::cell::RefCell<&'a mut dyn FnMut(usize)>,
}

impl Serialize for CountedMessages<'_> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeSeq;
        let mut seq = serializer.serialize_seq(Some(self.messages.len()))?;
        for (index, message) in self.messages.iter().enumerate() {
            seq.serialize_element(message)?;
            (self.on_message.borrow_mut())(index + 1);
        }
        seq.end()
    }
}

#[derive(Serialize)]
struct JsonConversation<'a> {
    title: &'a str,
    chat_identifier: &'a str,
    is_group: bool,
    message_count: usize,
}

pub fn write_json(conversation: &ExportConversation, out: &mut dyn Write) -> std::io::Result<()> {
    write_json_with_progress(conversation, out, &mut |_| {})
}

/// Like [`write_json`], calling `on_message(n)` after writing the n-th message.
pub fn write_json_with_progress(
    conversation: &ExportConversation,
    out: &mut dyn Write,
    on_message: &mut dyn FnMut(usize),
) -> std::io::Result<()> {
    let export = JsonExport {
        format: "amber-export",
        conversation: JsonConversation {
            title: &conversation.title,
            chat_identifier: &conversation.chat_identifier,
            is_group: conversation.is_group,
            message_count: conversation.messages.len(),
        },
        messages: CountedMessages {
            messages: &conversation.messages,
            on_message: std::cell::RefCell::new(on_message),
        },
    };
    serde_json::to_writer_pretty(&mut *out, &export).map_err(std::io::Error::other)?;
    writeln!(out)
}

// --- HTML -------------------------------------------------------------

/// Where an attachment lives next to the exported HTML, as relative URLs.
#[derive(Debug, Clone)]
pub struct MediaLinks {
    /// The file exactly as stored in the archive.
    pub original: String,
    /// A version a browser can show inline (e.g. a JPEG of a HEIC photo),
    /// when the original isn't one.
    pub preview: Option<String>,
}

fn escape_html(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            other => escaped.push(other),
        }
    }
    escaped
}

/// Timeline scrollbar styles and script for the HTML export (see
/// `export_timeline.js`). Plain files so they can be read and linted as what
/// they are.
const HTML_TIMELINE_STYLE: &str = include_str!("export_timeline.css");
const HTML_TIMELINE_SCRIPT: &str = include_str!("export_timeline.js");

const HTML_STYLE: &str = "
body{font:15px/1.4 -apple-system,'Segoe UI',Roboto,sans-serif;margin:0;background:#f2f2f7;color:#111}
header{position:sticky;top:0;background:#fffc;backdrop-filter:blur(8px);padding:12px 20px;border-bottom:1px solid #ddd}
header h1{font-size:17px;margin:0}header p{margin:2px 0 0;color:#666;font-size:12px}
main{max-width:720px;margin:0 auto;padding:12px 16px 60px}
.date{text-align:center;color:#8e8e93;font-size:12px;margin:22px 0 8px}
.msg{display:flex;flex-direction:column;margin:2px 0}.msg.me{align-items:flex-end}.msg.them{align-items:flex-start}
.who{font-size:11px;color:#8e8e93;margin:6px 10px 1px}
.bubble{max-width:80%;padding:8px 12px;border-radius:18px;white-space:pre-wrap;word-wrap:break-word}
.me .bubble{background:#0a84ff;color:#fff}.them .bubble{background:#e5e5ea}
.bubble img,.bubble video{display:block;max-width:100%;max-height:360px;border-radius:12px;margin:2px 0}
.bubble a{color:inherit}
.meta{font-size:11px;color:#8e8e93;margin:1px 10px}
.react{font-size:12px;margin:0 10px}
@media(prefers-color-scheme:dark){body{background:#000;color:#eee}header{background:#000c;border-color:#333}.them .bubble{background:#2c2c2e;color:#eee}}
";

fn is_image(mime: Option<&str>) -> bool {
    mime.is_some_and(|m| m.starts_with("image/"))
}

fn is_video(mime: Option<&str>) -> bool {
    mime.is_some_and(|m| m.starts_with("video/"))
}

fn is_audio(mime: Option<&str>) -> bool {
    mime.is_some_and(|m| m.starts_with("audio/"))
}

/// `media` returns where an attachment was copied to, or `None` when media
/// isn't part of this export (the filename is shown as plain text instead).
pub fn write_html(
    conversation: &ExportConversation,
    clock: Clock<'_>,
    media: &dyn Fn(&ExportAttachment) -> Option<MediaLinks>,
    out: &mut dyn Write,
) -> std::io::Result<()> {
    write_html_with_progress(conversation, clock, media, out, &mut |_| {})
}

/// Like [`write_html`], calling `on_message(n)` as it reaches the n-th message.
pub fn write_html_with_progress(
    conversation: &ExportConversation,
    clock: Clock<'_>,
    media: &dyn Fn(&ExportAttachment) -> Option<MediaLinks>,
    out: &mut dyn Write,
    on_message: &mut dyn FnMut(usize),
) -> std::io::Result<()> {
    let title = escape_html(&conversation.title);
    writeln!(
        out,
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\">\
         <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
         <title>{title}</title><style>{HTML_STYLE}{HTML_TIMELINE_STYLE}</style></head><body>"
    )?;
    writeln!(
        out,
        "<header><h1>{title}</h1><p>{} messages - exported from an Amber archive</p></header><main>",
        conversation.messages.len()
    )?;

    let mut current_day = String::new();
    for (index, message) in conversation.messages.iter().enumerate() {
        on_message(index + 1);
        let moment = clock(message.timestamp_unix_ms);
        if moment.date != current_day {
            // One wrapper per day: the timeline scrollbar jumps between
            // these (and the browser can skip drawing the ones off-screen,
            // which is what keeps a 100,000-message page usable).
            if !current_day.is_empty() {
                writeln!(out, "</section>")?;
            }
            writeln!(
                out,
                "<section class=\"day\" data-ts=\"{}\"><div class=\"date\">{}</div>",
                message.timestamp_unix_ms,
                escape_html(&moment.date)
            )?;
            current_day = moment.date.clone();
        }

        let side = if message.is_from_me { "me" } else { "them" };
        writeln!(out, "<div class=\"msg {side}\">")?;
        if conversation.is_group && !message.is_from_me {
            writeln!(
                out,
                "<div class=\"who\">{}</div>",
                escape_html(&message.sender)
            )?;
        }

        let has_body = !message.attachments.is_empty()
            || message.unsent
            || message.text.as_deref().is_some_and(|t| !t.is_empty());
        if has_body {
            write!(out, "<div class=\"bubble\">")?;
            if message.unsent {
                write!(out, "<em>This message was unsent.</em>")?;
            }
            for attachment in &message.attachments {
                write_html_attachment(out, attachment, media(attachment))?;
            }
            if let Some(text) = message
                .text
                .as_deref()
                .filter(|t| !t.is_empty() && !message.unsent)
            {
                write!(out, "{}", escape_html(text))?;
            }
            writeln!(out, "</div>")?;
        }

        let time = moment.time.get(..5).unwrap_or(&moment.time);
        let edited = if message.edited { " - edited" } else { "" };
        writeln!(
            out,
            "<div class=\"meta\">{}{edited}</div>",
            escape_html(time)
        )?;
        if !message.reactions.is_empty() {
            let reactions = message
                .reactions
                .iter()
                .map(|r| escape_html(r.emoji.as_deref().unwrap_or(&r.kind)))
                .collect::<Vec<_>>()
                .join(" ");
            writeln!(out, "<div class=\"react\">{reactions}</div>")?;
        }
        writeln!(out, "</div>")?;
    }

    if !current_day.is_empty() {
        writeln!(out, "</section>")?;
    }
    writeln!(out, "</main>")?;
    // The timeline scrollbar. Hidden until its script runs, so the page is
    // still a plain, scrollable document without JavaScript. The script is a
    // fixed string - nothing from the conversation is ever placed inside it.
    writeln!(
        out,
        "<div id=\"timeline\"><div id=\"tl-track\" role=\"slider\" tabindex=\"0\" \n         aria-label=\"Timeline\" aria-orientation=\"vertical\">\n         <div id=\"tl-thumb\"><div id=\"tl-label\" aria-hidden=\"true\"></div>\n         <div id=\"tl-handle\"></div></div></div></div>"
    )?;
    writeln!(out, "<script>{HTML_TIMELINE_SCRIPT}</script></body></html>")
}

fn write_html_attachment(
    out: &mut dyn Write,
    attachment: &ExportAttachment,
    links: Option<MediaLinks>,
) -> std::io::Result<()> {
    let name = escape_html(&attachment.filename);
    let Some(links) = links else {
        return write!(out, "<div>[{name}]</div>");
    };
    let original = escape_html(&links.original);
    let mime = attachment.mime_type.as_deref();

    if let Some(preview) = links.preview.as_deref().filter(|_| is_image(mime)) {
        // A browser-friendly copy is shown; the original stays one click away.
        return write!(
            out,
            "<a href=\"{original}\"><img src=\"{}\" alt=\"{name}\" loading=\"lazy\"></a>",
            escape_html(preview)
        );
    }
    if is_image(mime) {
        return write!(
            out,
            "<a href=\"{original}\"><img src=\"{original}\" alt=\"{name}\" loading=\"lazy\"></a>"
        );
    }
    if is_video(mime) {
        return write!(
            out,
            "<video src=\"{original}\" controls preload=\"none\"></video>"
        );
    }
    if is_audio(mime) {
        return write!(
            out,
            "<audio src=\"{original}\" controls preload=\"none\"></audio>"
        );
    }
    write!(out, "<div><a href=\"{original}\">{name}</a></div>")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> ExportConversation {
        ExportConversation {
            title: "Alice <&> Co".to_string(),
            chat_identifier: "+15551234567".to_string(),
            is_group: false,
            messages: vec![
                ExportMessage {
                    guid: Some("g1".into()),
                    // 2024-03-05T14:02:11Z
                    timestamp_unix_ms: 1_709_647_331_000,
                    timestamp_utc: "2024-03-05T14:02:11.000Z".into(),
                    sender: "Alice".into(),
                    is_from_me: false,
                    text: Some("hello, \"world\"\nsecond line".into()),
                    reply_to_guid: None,
                    edited: true,
                    unsent: false,
                    edit_history: None,
                    attachments: vec![ExportAttachment {
                        filename: "IMG_0001.HEIC".into(),
                        mime_type: Some("image/heic".into()),
                        byte_size: Some(1234),
                        sha256: "abc".into(),
                        rel_path: "attachments/ab/abc.heic".into(),
                    }],
                    reactions: vec![ExportReaction {
                        from: Some("Me".into()),
                        kind: "loved".into(),
                        emoji: None,
                    }],
                },
                ExportMessage {
                    guid: Some("g2".into()),
                    timestamp_unix_ms: 1_709_733_731_000, // next day
                    timestamp_utc: "2024-03-06T14:02:11.000Z".into(),
                    sender: "Me".into(),
                    is_from_me: true,
                    text: Some("<script>alert(1)</script>".into()),
                    reply_to_guid: None,
                    edited: false,
                    unsent: false,
                    edit_history: None,
                    attachments: vec![],
                    reactions: vec![],
                },
            ],
        }
    }

    fn to_string(f: impl FnOnce(&mut Vec<u8>) -> std::io::Result<()>) -> String {
        let mut buffer = Vec::new();
        f(&mut buffer).unwrap();
        String::from_utf8(buffer).unwrap()
    }

    #[test]
    fn text_has_day_headings_indented_continuations_and_attachment_lines() {
        let text = to_string(|out| write_text(&sample(), &utc_clock, out));

        assert!(text.contains("=== 2024-03-05 ==="));
        assert!(text.contains("=== 2024-03-06 ==="));
        assert!(text.contains("[14:02] Alice: hello, \"world\"\n    second line\n    (edited)"));
        assert!(text.contains("    [attachment: IMG_0001.HEIC]"));
        assert!(text.contains("    [reaction: loved from Me]"));
    }

    #[test]
    fn csv_quotes_fields_with_commas_quotes_and_newlines() {
        let csv = to_string(|out| write_csv(&sample(), &utc_clock, out));

        assert!(csv.starts_with('\u{feff}'));
        assert!(csv.contains("timestamp_utc,date,time,sender,direction,text"));
        assert!(csv.contains("\"hello, \"\"world\"\"\nsecond line\""));
        assert!(csv.contains(",received,"));
        assert!(csv.contains(",sent,"));
        assert!(csv.contains("IMG_0001.HEIC"));
    }

    #[test]
    fn json_is_valid_and_carries_the_structure() {
        let json = to_string(|out| write_json(&sample(), out));
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(value["format"], "amber-export");
        assert_eq!(value["conversation"]["message_count"], 2);
        assert_eq!(value["messages"][0]["sender"], "Alice");
        assert_eq!(
            value["messages"][0]["attachments"][0]["filename"],
            "IMG_0001.HEIC"
        );
        assert_eq!(value["messages"][1]["is_from_me"], true);
    }

    #[test]
    fn html_escapes_user_text_and_titles() {
        let html = to_string(|out| write_html(&sample(), &utc_clock, &|_| None, out));

        assert!(html.contains("<title>Alice &lt;&amp;&gt; Co</title>"));
        // A message containing markup must not become markup.
        assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
        assert!(!html.contains("<script>alert"));
        // Without media, attachments are just named.
        assert!(html.contains("[IMG_0001.HEIC]"));
    }

    #[test]
    fn html_wraps_each_day_in_a_section_for_the_timeline() {
        let html = to_string(|out| write_html(&sample(), &utc_clock, &|_| None, out));

        // The sample spans two days: two balanced sections, each carrying the
        // timestamp of its first message for the timeline to read. (Only the
        // page itself is counted - the script's comments mention the tag too.)
        let page = html.split("<script>").next().unwrap();
        assert_eq!(page.matches("<section class=\"day\"").count(), 2);
        assert_eq!(page.matches("</section>").count(), 2);
        assert!(html.contains("<section class=\"day\" data-ts=\"1709647331000\">"));
        assert!(html.contains("<section class=\"day\" data-ts=\"1709733731000\">"));
        assert!(html.contains("id=\"tl-track\""));
    }

    #[test]
    fn html_timeline_script_is_ours_alone_and_the_page_works_without_it() {
        let html = to_string(|out| write_html(&sample(), &utc_clock, &|_| None, out));

        // Exactly one script element - the exporter's own. A message that
        // contains "<script>" is escaped text, never a second script.
        assert_eq!(html.matches("<script>").count(), 1);
        assert!(html.contains("\"use strict\""));
        // Hidden until the script runs, so a no-JS reader still gets a normal page.
        assert!(html.contains("#timeline{display:none}"));
        assert!(html.contains("html.has-timeline #timeline{display:block"));
    }

    #[test]
    fn html_for_an_empty_conversation_is_still_a_valid_page() {
        let empty = ExportConversation {
            messages: vec![],
            ..sample()
        };
        let html = to_string(|out| write_html(&empty, &utc_clock, &|_| None, out));

        assert!(html.contains("0 messages"));
        assert_eq!(
            html.split("<script>")
                .next()
                .unwrap()
                .matches("<section")
                .count(),
            0
        );
        assert!(html.ends_with(
            "</script></body></html>
"
        ));
    }

    #[test]
    fn html_links_media_and_prefers_a_browser_friendly_preview() {
        let html = to_string(|out| {
            write_html(
                &sample(),
                &utc_clock,
                &|_| {
                    Some(MediaLinks {
                        original: "files/abc.heic".into(),
                        preview: Some("files/abc.jpg".into()),
                    })
                },
                out,
            )
        });

        assert!(html.contains("<a href=\"files/abc.heic\"><img src=\"files/abc.jpg\""));
    }
}
