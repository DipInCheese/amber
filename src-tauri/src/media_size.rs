// SPDX-License-Identifier: GPL-3.0-or-later
//! Reads how big a photo or video looks on screen, so the viewer can reserve
//! a frame of the right shape before the picture itself has loaded.
//!
//! "Looks" matters: an iPhone stores most portrait photos sideways and adds
//! a rotation note (EXIF orientation in a JPEG, an `irot` box in a HEIC, a
//! rotation matrix in a video). The size to report is the one after that
//! rotation, because that is what the viewer ends up drawing.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

/// How much of an image's start is searched for its rotation note.
const IMAGE_HEAD_BYTES: u64 = 512 * 1024;
/// A video's `moov` box is normally well under a megabyte; refuse nonsense.
const MAX_MOOV_BYTES: u64 = 64 * 1024 * 1024;

/// Displayed pixel size of the photo or video at `path`, if it can be
/// worked out. `None` just means "unknown" - the viewer falls back to a
/// default frame.
pub fn display_size(path: &Path, mime_type: Option<&str>) -> Option<(u32, u32)> {
    let mime = mime_type.unwrap_or_default();
    if mime.starts_with("image/") {
        image_size(path)
    } else if mime.starts_with("video/") {
        video_size(path)
    } else {
        None
    }
}

fn image_size(path: &Path) -> Option<(u32, u32)> {
    let size = imagesize::size(path).ok()?;
    let (mut width, mut height) = (
        u32::try_from(size.width).ok()?,
        u32::try_from(size.height).ok()?,
    );
    if width == 0 || height == 0 {
        return None;
    }

    let mut head = Vec::new();
    File::open(path)
        .ok()?
        .take(IMAGE_HEAD_BYTES)
        .read_to_end(&mut head)
        .ok()?;
    if turns_a_quarter(&head) {
        std::mem::swap(&mut width, &mut height);
    }
    Some((width, height))
}

/// Whether the picture is stored sideways (so its width and height swap on screen).
fn turns_a_quarter(head: &[u8]) -> bool {
    if let Some(orientation) = jpeg_orientation(head) {
        return (5..=8).contains(&orientation);
    }
    heif_quarter_turn(head)
}

/// The EXIF orientation (1-8) of a JPEG, if it has one.
fn jpeg_orientation(data: &[u8]) -> Option<u16> {
    if data.get(0..2)? != [0xFF, 0xD8] {
        return None;
    }
    let mut at = 2;
    while at + 4 <= data.len() {
        if data[at] != 0xFF {
            return None;
        }
        let marker = data[at + 1];
        if marker == 0xFF {
            at += 1;
            continue;
        }
        // Start of scan / end of image: no more metadata segments.
        if marker == 0xDA || marker == 0xD9 {
            return None;
        }
        let length = usize::from(u16::from_be_bytes([data[at + 2], data[at + 3]]));
        let segment = data.get(at + 4..at + 2 + length)?;
        if marker == 0xE1 && segment.starts_with(b"Exif\0\0") {
            return tiff_orientation(&segment[6..]);
        }
        at += 2 + length;
    }
    None
}

fn tiff_orientation(tiff: &[u8]) -> Option<u16> {
    let little = match tiff.get(0..2)? {
        b"II" => true,
        b"MM" => false,
        _ => return None,
    };
    let u16_at = |at: usize| -> Option<u16> {
        let bytes: [u8; 2] = tiff.get(at..at + 2)?.try_into().ok()?;
        Some(if little {
            u16::from_le_bytes(bytes)
        } else {
            u16::from_be_bytes(bytes)
        })
    };
    let u32_at = |at: usize| -> Option<u32> {
        let bytes: [u8; 4] = tiff.get(at..at + 4)?.try_into().ok()?;
        Some(if little {
            u32::from_le_bytes(bytes)
        } else {
            u32::from_be_bytes(bytes)
        })
    };

    let ifd = usize::try_from(u32_at(4)?).ok()?;
    let entries = usize::from(u16_at(ifd)?);
    for index in 0..entries {
        let entry = ifd + 2 + index * 12;
        if u16_at(entry)? == 0x0112 {
            return u16_at(entry + 8);
        }
    }
    None
}

/// Whether a HEIC/HEIF carries an `irot` (rotation) box turning it by 90 or 270 degrees.
fn heif_quarter_turn(data: &[u8]) -> bool {
    if data.get(4..8) != Some(b"ftyp") {
        return false;
    }
    // An `irot` property is an 9-byte box: size (9), "irot", angle in the low two bits.
    const SIGNATURE: [u8; 8] = [0, 0, 0, 9, b'i', b'r', b'o', b't'];
    data.windows(9)
        .find(|window| window[..8] == SIGNATURE)
        .is_some_and(|window| window[8] & 1 == 1)
}

fn video_size(path: &Path) -> Option<(u32, u32)> {
    let mut file = File::open(path).ok()?;
    let length = file.metadata().ok()?.len();

    let mut position = 0u64;
    while position + 8 <= length {
        file.seek(SeekFrom::Start(position)).ok()?;
        let mut header = [0u8; 8];
        file.read_exact(&mut header).ok()?;
        let size32 = u64::from(u32::from_be_bytes(header[0..4].try_into().ok()?));
        let kind = &header[4..8];

        let (size, header_len) = match size32 {
            1 => {
                let mut wide = [0u8; 8];
                file.read_exact(&mut wide).ok()?;
                (u64::from_be_bytes(wide), 16)
            }
            0 => (length - position, 8),
            n => (n, 8),
        };
        if size < header_len {
            return None;
        }

        if kind == b"moov" {
            let payload = size - header_len;
            if payload > MAX_MOOV_BYTES {
                return None;
            }
            let mut moov = vec![0u8; usize::try_from(payload).ok()?];
            file.read_exact(&mut moov).ok()?;
            return moov_size(&moov);
        }
        position += size;
    }
    None
}

/// The boxes inside `data`, as (type, payload) pairs.
fn boxes(data: &[u8]) -> Vec<(&[u8], &[u8])> {
    let mut found = Vec::new();
    let mut at = 0usize;
    while at + 8 <= data.len() {
        let size =
            u32::from_be_bytes([data[at], data[at + 1], data[at + 2], data[at + 3]]) as usize;
        if size < 8 || at + size > data.len() {
            break;
        }
        found.push((&data[at + 4..at + 8], &data[at + 8..at + size]));
        at += size;
    }
    found
}

fn moov_size(moov: &[u8]) -> Option<(u32, u32)> {
    boxes(moov)
        .into_iter()
        .filter(|(kind, _)| *kind == b"trak")
        .flat_map(|(_, trak)| boxes(trak))
        .filter(|(kind, _)| *kind == b"tkhd")
        .find_map(|(_, tkhd)| track_size(tkhd))
}

/// Width and height from a track header, after its rotation matrix. Audio
/// tracks have a zero size and are skipped by the caller.
fn track_size(tkhd: &[u8]) -> Option<(u32, u32)> {
    let version = *tkhd.first()?;
    let matrix = if version == 1 { 52 } else { 40 };
    let read = |at: usize| -> Option<u32> {
        Some(u32::from_be_bytes(tkhd.get(at..at + 4)?.try_into().ok()?))
    };

    let width = read(matrix + 36)? >> 16;
    let height = read(matrix + 40)? >> 16;
    if width == 0 || height == 0 {
        return None;
    }
    // Matrix entries a and b are 16.16 fixed point; a portrait clip filmed
    // sideways has a = 0 and b != 0.
    let (a, b) = (read(matrix)?, read(matrix + 4)?);
    Some(if a == 0 && b != 0 {
        (height, width)
    } else {
        (width, height)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn mp4_box(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut bytes = ((payload.len() + 8) as u32).to_be_bytes().to_vec();
        bytes.extend_from_slice(kind);
        bytes.extend_from_slice(payload);
        bytes
    }

    fn tkhd(width: u32, height: u32, rotated: bool) -> Vec<u8> {
        let mut payload = vec![0u8; 4 + 4 + 4 + 4 + 4 + 4]; // version/flags .. duration
        payload.extend_from_slice(&[0u8; 8 + 8]); // reserved, layer, group, volume, reserved
        let (a, b) = if rotated {
            (0u32, 0x0001_0000u32)
        } else {
            (0x0001_0000, 0)
        };
        let matrix = [
            a,
            b,
            0,
            if rotated { 0xFFFF_0000 } else { 0 },
            if rotated { 0 } else { 0x0001_0000 },
            0,
            0,
            0,
            0x4000_0000,
        ];
        for value in matrix {
            payload.extend_from_slice(&value.to_be_bytes());
        }
        payload.extend_from_slice(&(width << 16).to_be_bytes());
        payload.extend_from_slice(&(height << 16).to_be_bytes());
        mp4_box(b"tkhd", &payload)
    }

    fn write_video(width: u32, height: u32, rotated: bool) -> tempfile::TempPath {
        let audio = mp4_box(b"trak", &tkhd(0, 0, false));
        let video = mp4_box(b"trak", &tkhd(width, height, rotated));
        let moov = mp4_box(b"moov", &[audio, video].concat());
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(&mp4_box(b"ftyp", b"qt  \0\0\0\0")).unwrap();
        file.write_all(&mp4_box(b"mdat", &[0u8; 100])).unwrap();
        file.write_all(&moov).unwrap();
        file.into_temp_path()
    }

    #[test]
    fn reads_a_videos_size_from_a_moov_at_the_end() {
        let path = write_video(1920, 1080, false);
        assert_eq!(
            display_size(&path, Some("video/quicktime")),
            Some((1920, 1080))
        );
    }

    #[test]
    fn a_video_filmed_sideways_is_reported_upright() {
        let path = write_video(1920, 1080, true);
        assert_eq!(display_size(&path, Some("video/mp4")), Some((1080, 1920)));
    }

    #[test]
    fn jpeg_orientation_is_read_from_exif() {
        // SOI, APP1 "Exif" with a little-endian TIFF whose only entry is Orientation = 6.
        let mut tiff = b"II*\0\x08\0\0\0".to_vec();
        tiff.extend_from_slice(&1u16.to_le_bytes());
        tiff.extend_from_slice(&0x0112u16.to_le_bytes());
        tiff.extend_from_slice(&3u16.to_le_bytes());
        tiff.extend_from_slice(&1u32.to_le_bytes());
        tiff.extend_from_slice(&6u16.to_le_bytes());
        tiff.extend_from_slice(&[0, 0]);
        let mut app1 = b"Exif\0\0".to_vec();
        app1.extend_from_slice(&tiff);
        let mut jpeg = vec![0xFF, 0xD8, 0xFF, 0xE1];
        jpeg.extend_from_slice(&((app1.len() + 2) as u16).to_be_bytes());
        jpeg.extend_from_slice(&app1);
        jpeg.extend_from_slice(&[0xFF, 0xD9]);

        assert_eq!(jpeg_orientation(&jpeg), Some(6));
        assert!(turns_a_quarter(&jpeg));
        assert_eq!(jpeg_orientation(&[0xFF, 0xD8, 0xFF, 0xD9]), None);
    }

    #[test]
    fn heic_rotation_swaps_the_axes_only_for_quarter_turns() {
        let mut heic = vec![0, 0, 0, 24];
        heic.extend_from_slice(b"ftypheic");
        heic.extend_from_slice(&[0u8; 12]);
        assert!(!heif_quarter_turn(&heic));

        let mut turned = heic.clone();
        turned.extend_from_slice(&[0, 0, 0, 9, b'i', b'r', b'o', b't', 3]);
        assert!(heif_quarter_turn(&turned));

        let mut upside_down = heic;
        upside_down.extend_from_slice(&[0, 0, 0, 9, b'i', b'r', b'o', b't', 2]);
        assert!(!heif_quarter_turn(&upside_down));
    }

    #[test]
    fn reads_a_png_size_and_ignores_other_kinds() {
        // PNG signature + IHDR for a 640x480 image.
        let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 13];
        png.extend_from_slice(b"IHDR");
        png.extend_from_slice(&640u32.to_be_bytes());
        png.extend_from_slice(&480u32.to_be_bytes());
        png.extend_from_slice(&[8, 6, 0, 0, 0, 0, 0, 0, 0]);
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(&png).unwrap();
        let path = file.into_temp_path();

        assert_eq!(display_size(&path, Some("image/png")), Some((640, 480)));
        assert_eq!(display_size(&path, Some("audio/mp4")), None);
        assert_eq!(display_size(&path, None), None);
    }
}
