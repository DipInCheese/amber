// SPDX-License-Identifier: GPL-3.0-or-later
//! Serves attachment bytes straight out of the open `.amber` archive's ZIP
//! over a custom `amber-attachment://` URI scheme, so the frontend can point
//! `<img>`/`<video>` tags directly at attachment paths without Amber ever
//! copying media out into the web layer.

use std::sync::{Arc, Mutex, OnceLock};

use tauri::{http, AppHandle, Manager, UriSchemeContext, UriSchemeResponder, Wry};

use crate::commands::AppState;
use crate::heic;
use crate::mime::guess_from_extension;
use crate::rendition_cache::RenditionCache;

/// Converted HEIC renditions (~0.5-1 MB each), most recent ones kept.
const RENDITION_CACHE_BYTES: usize = 192 * 1024 * 1024;

fn renditions() -> &'static Mutex<RenditionCache> {
    static CACHE: OnceLock<Mutex<RenditionCache>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(RenditionCache::new(RENDITION_CACHE_BYTES)))
}

pub const SCHEME: &str = "amber-attachment";

/// Registered as an *asynchronous* scheme: reading a large video out of the
/// archive or decoding a photo must not run on the thread that drives the
/// UI, or scrolling a conversation full of media would stutter.
pub fn handle_request(
    ctx: UriSchemeContext<'_, Wry>,
    request: http::Request<Vec<u8>>,
    responder: UriSchemeResponder,
) {
    let app = ctx.app_handle().clone();
    std::thread::spawn(move || responder.respond(respond(&app, &request)));
}

fn respond(app: &AppHandle, request: &http::Request<Vec<u8>>) -> http::Response<Vec<u8>> {
    // e.g. "/attachments/ab/abcdef....heic" -> "attachments/ab/abcdef....heic"
    let rel_path = request.uri().path().trim_start_matches('/');
    let rel_path = percent_decode(rel_path);

    // A photo converted a moment ago needs no archive read and no decoding.
    if heic::is_heic(&rel_path) {
        let cached = renditions().lock().unwrap().get(&rel_path);
        if let Some(jpeg) = cached {
            return ok_response(jpeg.to_vec(), "image/jpeg");
        }
    }

    // Hold the archive lock only for the read itself, not for any decoding.
    let read = {
        let state = app.state::<AppState>();
        let guard = state.archive();
        let Some(archive) = guard.as_ref() else {
            return not_found("no archive is open");
        };
        archive.read_attachment(&rel_path)
    };
    let bytes = match read {
        Ok(bytes) => bytes,
        Err(err) => return not_found(&err.to_string()),
    };

    let (bytes, content_type) = displayable(&rel_path, bytes);
    if content_type == "image/jpeg" && heic::is_heic(&rel_path) {
        renditions()
            .lock()
            .unwrap()
            .insert(rel_path.clone(), Arc::new(bytes.clone()));
    }
    ok_response(bytes, content_type)
}

fn ok_response(bytes: Vec<u8>, content_type: &str) -> http::Response<Vec<u8>> {
    http::Response::builder()
        .status(http::StatusCode::OK)
        .header(http::header::CONTENT_TYPE, content_type)
        .header(http::header::CONTENT_LENGTH, bytes.len())
        // Paths are content-addressed (they contain the file's SHA-256), so
        // the bytes behind a URL never change.
        .header(
            http::header::CACHE_CONTROL,
            "public, max-age=31536000, immutable",
        )
        .body(bytes)
        .unwrap_or_else(|_| not_found("failed to build response"))
}

/// The bytes and content type to actually send. Everything is served as
/// stored, except HEIC photos where the webview can't decode them (see
/// `heic.rs`): those get a JPEG rendition if one can be made, and fall back
/// to the original bytes - which the viewer then shows as a plain file - if
/// it can't.
fn displayable(rel_path: &str, bytes: Vec<u8>) -> (Vec<u8>, &'static str) {
    if heic::is_heic(rel_path) {
        if let Some(jpeg) = heic::to_jpeg(&bytes) {
            return (jpeg, "image/jpeg");
        }
    }
    (bytes, guess_from_extension(rel_path))
}

fn not_found(message: &str) -> http::Response<Vec<u8>> {
    http::Response::builder()
        .status(http::StatusCode::NOT_FOUND)
        .header(http::header::CONTENT_TYPE, "text/plain")
        .body(message.as_bytes().to_vec())
        .expect("static response is always valid")
}

/// Decodes the handful of percent-escapes that can show up in a `rel_path`
/// (attachment filenames are sha256 hex + a simple extension, so this is a
/// defensive minimum, not a general-purpose decoder).
fn percent_decode(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut bytes = input.bytes().peekable();

    while let Some(byte) = bytes.next() {
        if byte == b'%' {
            let hi = bytes.next();
            let lo = bytes.next();
            if let (Some(hi), Some(lo)) = (hi, lo) {
                if let (Some(hi), Some(lo)) = ((hi as char).to_digit(16), (lo as char).to_digit(16))
                {
                    output.push(((hi * 16 + lo) as u8) as char);
                    continue;
                }
            }
            output.push('%');
        } else {
            output.push(byte as char);
        }
    }

    output
}
