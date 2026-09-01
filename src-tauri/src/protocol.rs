// SPDX-License-Identifier: GPL-3.0-or-later
//! Serves attachment bytes straight out of the open `.amber` archive's ZIP
//! over a custom `amber-attachment://` URI scheme, so the frontend can point
//! `<img>`/`<video>` tags directly at attachment paths without Amber ever
//! copying media out into the web layer.

use tauri::{http, Manager, UriSchemeContext, Wry};

use crate::commands::AppState;
use crate::mime::guess_from_extension;

pub const SCHEME: &str = "amber-attachment";

pub fn handle_request(
    ctx: UriSchemeContext<'_, Wry>,
    request: http::Request<Vec<u8>>,
) -> http::Response<Vec<u8>> {
    // e.g. "/attachments/ab/abcdef....heic" -> "attachments/ab/abcdef....heic"
    let rel_path = request.uri().path().trim_start_matches('/');
    let rel_path = percent_decode(rel_path);

    let state = ctx.app_handle().state::<AppState>();
    let guard = state.archive();
    let Some(archive) = guard.as_ref() else {
        return not_found("no archive is open");
    };

    match archive.read_attachment(&rel_path) {
        Ok(bytes) => http::Response::builder()
            .status(http::StatusCode::OK)
            .header(http::header::CONTENT_TYPE, guess_from_extension(&rel_path))
            .header(http::header::CONTENT_LENGTH, bytes.len())
            .body(bytes)
            .unwrap_or_else(|_| not_found("failed to build response")),
        Err(err) => not_found(&err.to_string()),
    }
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
