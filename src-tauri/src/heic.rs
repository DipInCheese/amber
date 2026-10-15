// SPDX-License-Identifier: GPL-3.0-or-later
//! Display-only HEIC -> JPEG conversion for the viewer.
//!
//! iPhone photos are usually HEIC. macOS's webview shows them natively, but
//! WebView2 (Chromium) cannot decode HEIC at all - even with the Windows
//! HEIF extension installed - so an inline `<img>` would just be broken.
//! Windows itself *can* decode it through the Windows Imaging Component when
//! the free "HEIF Image Extensions" package is present, so on Windows the
//! attachment protocol hands the webview a JPEG rendition of the photo.
//!
//! The archive always keeps the original HEIC bytes (durability: nothing is
//! ever re-encoded on disk); this only changes what the webview is served.
//! Returns `None` wherever conversion isn't possible (not Windows, no HEIF
//! codec, a corrupt file) and the caller falls back to the original bytes.

/// Longest edge, in pixels, of the rendition sent to the webview. A phone
/// photo is 12+ MP; a chat bubble never needs that.
pub const MAX_EDGE: u32 = 2048;

pub fn is_heic(rel_path: &str) -> bool {
    let lower = rel_path.to_ascii_lowercase();
    lower.ends_with(".heic") || lower.ends_with(".heif")
}

#[cfg(windows)]
pub fn to_jpeg(bytes: &[u8]) -> Option<Vec<u8>> {
    wic::convert(bytes, MAX_EDGE).ok()
}

#[cfg(not(windows))]
pub fn to_jpeg(_bytes: &[u8]) -> Option<Vec<u8>> {
    None
}

#[cfg(windows)]
mod wic {
    use windows::core::{Interface, GUID};
    use windows::Win32::Foundation::{E_FAIL, HGLOBAL};
    use windows::Win32::Graphics::Imaging::{
        CLSID_WICImagingFactory, GUID_ContainerFormatJpeg, GUID_WICPixelFormat24bppBGR,
        IWICBitmapFrameEncode, IWICBitmapSource, IWICImagingFactory, WICBitmapDitherTypeNone,
        WICBitmapEncoderNoCache, WICBitmapInterpolationModeFant, WICBitmapPaletteTypeCustom,
        WICDecodeMetadataCacheOnDemand,
    };
    use windows::Win32::System::Com::StructuredStorage::{
        CreateStreamOnHGlobal, GetHGlobalFromStream,
    };
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, IStream, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
    };
    use windows::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};

    pub fn convert(bytes: &[u8], max_edge: u32) -> windows::core::Result<Vec<u8>> {
        // SAFETY: plain COM calls on interfaces created in this function;
        // the HGLOBAL behind `output` is locked only while it's copied out.
        unsafe {
            // Already-initialised threads report an error here; COM is
            // usable either way.
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);

            let factory: IWICImagingFactory =
                CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)?;

            let input = factory.CreateStream()?;
            input.InitializeFromMemory(bytes)?;
            let input: IStream = input.cast()?;
            let decoder = factory.CreateDecoderFromStream(
                &input,
                std::ptr::null(),
                WICDecodeMetadataCacheOnDemand,
            )?;
            let frame = decoder.GetFrame(0)?;

            let (mut width, mut height) = (0u32, 0u32);
            frame.GetSize(&mut width, &mut height)?;
            let mut source: IWICBitmapSource = frame.cast()?;

            let longest = width.max(height);
            if longest > max_edge {
                let scale = f64::from(max_edge) / f64::from(longest);
                width = ((f64::from(width) * scale).round() as u32).max(1);
                height = ((f64::from(height) * scale).round() as u32).max(1);
                let scaler = factory.CreateBitmapScaler()?;
                scaler.Initialize(&source, width, height, WICBitmapInterpolationModeFant)?;
                source = scaler.cast()?;
            }

            // JPEG has no alpha and wants plain 24-bit BGR.
            let converter = factory.CreateFormatConverter()?;
            converter.Initialize(
                &source,
                &GUID_WICPixelFormat24bppBGR,
                WICBitmapDitherTypeNone,
                None,
                0.0,
                WICBitmapPaletteTypeCustom,
            )?;

            let output = CreateStreamOnHGlobal(HGLOBAL::default(), true)?;
            let encoder = factory.CreateEncoder(&GUID_ContainerFormatJpeg, std::ptr::null())?;
            encoder.Initialize(&output, WICBitmapEncoderNoCache)?;

            let mut frame_out: Option<IWICBitmapFrameEncode> = None;
            encoder.CreateNewFrame(&mut frame_out, std::ptr::null_mut())?;
            let frame_out = frame_out.ok_or_else(|| windows::core::Error::from(E_FAIL))?;
            frame_out.Initialize(None)?;
            frame_out.SetSize(width, height)?;
            let mut format: GUID = GUID_WICPixelFormat24bppBGR;
            frame_out.SetPixelFormat(&mut format)?;
            frame_out.WriteSource(&converter, std::ptr::null())?;
            frame_out.Commit()?;
            encoder.Commit()?;

            let hglobal = GetHGlobalFromStream(&output)?;
            let size = GlobalSize(hglobal);
            let pointer = GlobalLock(hglobal) as *const u8;
            if pointer.is_null() {
                return Err(E_FAIL.into());
            }
            let jpeg = std::slice::from_raw_parts(pointer, size).to_vec();
            let _ = GlobalUnlock(hglobal);
            Ok(jpeg)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_heic_and_heif_extensions_case_insensitively() {
        assert!(is_heic("attachments/ab/abcdef.heic"));
        assert!(is_heic("attachments/ab/abcdef.HEIC"));
        assert!(is_heic("attachments/ab/abcdef.heif"));
        assert!(!is_heic("attachments/ab/abcdef.jpeg"));
        assert!(!is_heic("attachments/ab/abcdef.bin"));
    }

    #[test]
    fn garbage_bytes_fall_back_instead_of_panicking() {
        assert!(to_jpeg(b"definitely not a HEIC file").is_none());
    }
}
