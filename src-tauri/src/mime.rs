// SPDX-License-Identifier: GPL-3.0-or-later
//! Minimal extension -> MIME type mapping for serving attachment bytes over
//! the `amber-attachment` protocol. Covers what iMessage/SMS attachments
//! actually are; not a general-purpose MIME database.

pub fn guess_from_extension(rel_path: &str) -> &'static str {
    let ext = rel_path
        .rsplit_once('.')
        .map(|(_, ext)| ext.to_ascii_lowercase())
        .unwrap_or_default();

    match ext.as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "heic" => "image/heic",
        "heics" => "image/heic-sequence",
        "tiff" | "tif" => "image/tiff",
        "mp4" => "video/mp4",
        "mov" => "video/quicktime",
        "m4v" => "video/x-m4v",
        "m4a" => "audio/mp4",
        "caf" => "audio/x-caf",
        "amr" => "audio/amr",
        "pdf" => "application/pdf",
        _ => "application/octet-stream",
    }
}
