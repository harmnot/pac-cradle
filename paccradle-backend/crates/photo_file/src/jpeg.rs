//! JPEG: a run of segments, each `FF <marker>` and, for most, a 2-byte length; the image data
//! follows each start-of-scan (SOS) up to the next marker.

use crate::bytes::{be_u16, byte, slice};
use crate::exif::{self, EXIF_ID};
use crate::{ImageFormat, ImageInfo, PhotoError};

const SOI: u8 = 0xD8;
const EOI: u8 = 0xD9;
const SOS: u8 = 0xDA;
const APP1: u8 = 0xE1;
const APP2: u8 = 0xE2;
const APP14: u8 = 0xEE;
const COM: u8 = 0xFE;

/// The size, from the first start-of-frame segment.
pub(crate) fn size(bytes: &[u8]) -> Result<ImageInfo, PhotoError> {
    let mut at = 2;
    loop {
        let (marker, after) = marker((bytes, at))?;
        at = after;
        match marker {
            // Markers that stand alone, with no length.
            0x01 | 0xD0..=0xD7 => continue,
            // Image data or the end before any frame: there's no size to read.
            EOI | SOS => return Err(PhotoError::UnreadableImage),
            // Start of frame (every kind but the DHT, JPG and DAC markers among them).
            0xC0..=0xCF if !matches!(marker, 0xC4 | 0xC8 | 0xCC) => {
                return Ok(ImageInfo {
                    format: ImageFormat::Jpeg,
                    height: u32::from(be_u16((bytes, at + 3))?),
                    width: u32::from(be_u16((bytes, at + 5))?),
                });
            }
            _ => at = segment_end((bytes, at))?,
        }
    }
}

/// Copies the segments that draw the picture (tables, frame, scans) and its colour profile
/// (ICC in APP2, Adobe's colour transform in APP14). Every other APPn segment (EXIF, XMP,
/// JFIF and its thumbnail, IPTC, MPF…) and comments are left out, and so is anything after
/// the end marker. An orientation other than 1 comes back as a fresh EXIF segment holding
/// only that, right after the start marker.
pub(crate) fn clean(bytes: &[u8]) -> Result<Vec<u8>, PhotoError> {
    let mut kept = Vec::with_capacity(bytes.len());
    let mut orientation = None;
    let mut at = 2;
    loop {
        let (marker, after) = marker((bytes, at))?;
        match marker {
            EOI => {
                kept.extend([0xFF, EOI]);
                break;
            }
            SOI => return Err(PhotoError::UnreadableImage),
            0x01 | 0xD0..=0xD7 => {
                kept.extend([0xFF, marker]);
                at = after;
            }
            SOS => {
                let header_end = segment_end((bytes, after))?;
                let data_end = scan_end((bytes, header_end))?;
                kept.extend([0xFF, SOS]);
                kept.extend(slice(bytes, (after, data_end))?);
                at = data_end;
            }
            _ => {
                let end = segment_end((bytes, after))?;
                let payload = slice(bytes, (after + 2, end))?;
                if marker == APP1 && orientation.is_none() && payload.starts_with(EXIF_ID) {
                    orientation = exif::orientation(payload);
                }
                if is_kept(marker, payload) {
                    kept.extend([0xFF, marker]);
                    kept.extend(slice(bytes, (after, end))?);
                }
                at = end;
            }
        }
    }
    let mut out = Vec::with_capacity(kept.len() + 40);
    out.extend([0xFF, SOI]);
    if let Some(orientation) = exif::turned(orientation) {
        out.extend(orientation_segment(orientation));
    }
    out.extend(kept);
    Ok(out)
}

/// Whether a segment with a length is kept.
fn is_kept(marker: u8, payload: &[u8]) -> bool {
    match marker {
        APP2 => payload.starts_with(b"ICC_PROFILE\0"),
        APP14 => payload.starts_with(b"Adobe"),
        0xE0..=0xEF | COM => false,
        // Tables, frames, restart intervals: the picture itself.
        _ => true,
    }
}

/// An APP1 EXIF segment holding only `orientation`.
fn orientation_segment(orientation: u16) -> Vec<u8> {
    let tiff = exif::orientation_tiff(orientation);
    // The length counts itself (2 bytes), the EXIF marker and the TIFF block: 34 in all.
    let length = (2 + EXIF_ID.len() + tiff.len()) as u16;
    let mut segment = vec![0xFF, APP1];
    segment.extend(length.to_be_bytes());
    segment.extend(EXIF_ID);
    segment.extend(tiff);
    segment
}

/// The marker at `at` (after any 0xFF fill bytes) and where its segment's body starts.
fn marker((bytes, at): (&[u8], usize)) -> Result<(u8, usize), PhotoError> {
    if byte((bytes, at))? != 0xFF {
        return Err(PhotoError::UnreadableImage);
    }
    let mut at = at + 1;
    while byte((bytes, at))? == 0xFF {
        at += 1;
    }
    Ok((byte((bytes, at))?, at + 1))
}

/// Where a segment whose length starts at `at` ends.
fn segment_end((bytes, at): (&[u8], usize)) -> Result<usize, PhotoError> {
    let length = usize::from(be_u16((bytes, at))?);
    if length < 2 || at + length > bytes.len() {
        return Err(PhotoError::UnreadableImage);
    }
    Ok(at + length)
}

/// Where the image data starting at `at` ends: at the first marker that isn't a stuffed
/// 0xFF (`FF 00`) or a restart marker (`FF D0`…`FF D7`). Data with no marker after it was
/// cut short.
fn scan_end((bytes, mut at): (&[u8], usize)) -> Result<usize, PhotoError> {
    loop {
        let rest = bytes.get(at..).ok_or(PhotoError::UnreadableImage)?;
        let ff = at
            + rest
                .iter()
                .position(|&value| value == 0xFF)
                .ok_or(PhotoError::UnreadableImage)?;
        let mut next = ff + 1;
        while byte((bytes, next))? == 0xFF {
            next += 1;
        }
        match byte((bytes, next))? {
            0x00 | 0xD0..=0xD7 => at = next + 1,
            _ => return Ok(ff),
        }
    }
}
