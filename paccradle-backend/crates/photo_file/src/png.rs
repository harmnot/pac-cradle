//! PNG: the signature, then chunks of a 4-byte length, a 4-byte type, the data and a CRC.

use crate::bytes::{be_u32, index, slice, tag};
use crate::exif;
use crate::{ImageFormat, ImageInfo, PhotoError};

pub(crate) const SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";

/// The chunks that show the picture: the image itself, its palette and transparency, and how
/// to read its colours. Text (`tEXt`, `zTXt`, `iTXt`, where XMP lives), `eXIf`, `tIME`,
/// animation and unknown chunks are left out.
const KEPT: [&[u8; 4]; 12] = [
    b"IHDR", b"PLTE", b"IDAT", b"IEND", b"tRNS", b"gAMA", b"cHRM", b"sRGB", b"iCCP", b"sBIT",
    b"cICP", b"pHYs",
];

/// The size, from the IHDR chunk that must come first.
pub(crate) fn size(bytes: &[u8]) -> Result<ImageInfo, PhotoError> {
    if bytes.get(12..16) != Some(b"IHDR") {
        return Err(PhotoError::UnreadableImage);
    }
    Ok(ImageInfo {
        format: ImageFormat::Png,
        width: be_u32((bytes, 16))?,
        height: be_u32((bytes, 20))?,
    })
}

/// Copies the [`KEPT`] chunks up to IEND, as they are. An orientation other than 1 from an
/// `eXIf` chunk comes back as a fresh `eXIf` holding only that, right after IHDR.
pub(crate) fn clean(bytes: &[u8]) -> Result<Vec<u8>, PhotoError> {
    let mut chunks = Vec::new();
    let mut orientation = None;
    let mut at = SIGNATURE.len();
    loop {
        let length = index(be_u32((bytes, at))?)?;
        let kind = tag((bytes, at + 4))?;
        let data_end = (at + 8)
            .checked_add(length)
            .ok_or(PhotoError::UnreadableImage)?;
        let data = slice(bytes, (at + 8, data_end))?;
        let chunk = slice(bytes, (at, data_end + 4))?;
        if &kind == b"eXIf" && orientation.is_none() {
            orientation = exif::orientation(data);
        }
        if KEPT.contains(&&kind) {
            chunks.push(chunk);
        }
        at = data_end + 4;
        if &kind == b"IEND" {
            break;
        }
    }
    let mut out = Vec::with_capacity(bytes.len());
    out.extend(SIGNATURE);
    let (ihdr, rest) = chunks.split_first().ok_or(PhotoError::UnreadableImage)?;
    out.extend(*ihdr);
    if let Some(orientation) = exif::turned(orientation) {
        out.extend(chunk(b"eXIf", &exif::orientation_tiff(orientation)));
    }
    for chunk in rest {
        out.extend(*chunk);
    }
    Ok(out)
}

/// A chunk of `kind` holding `data`, with its CRC.
fn chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(12 + data.len());
    // Only ever a few dozen bytes here.
    out.extend((data.len() as u32).to_be_bytes());
    out.extend(kind);
    out.extend(data);
    let mut crc = Crc32::new();
    crc.update(kind);
    crc.update(data);
    out.extend(crc.finish().to_be_bytes());
    out
}

/// CRC-32 as PNG uses it (ISO 3309, reflected polynomial 0xEDB88320).
struct Crc32(u32);

impl Crc32 {
    /// A CRC over nothing yet.
    fn new() -> Self {
        Self(0xFFFF_FFFF)
    }

    /// Adds `bytes`.
    fn update(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.0 ^= u32::from(byte);
            for _ in 0..8 {
                let carry = self.0 & 1;
                self.0 >>= 1;
                if carry == 1 {
                    self.0 ^= 0xEDB8_8320;
                }
            }
        }
    }

    /// The CRC of everything added.
    fn finish(&self) -> u32 {
        !self.0
    }
}
