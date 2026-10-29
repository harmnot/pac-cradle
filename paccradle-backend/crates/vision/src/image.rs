//! Reading an uploaded photo safely (CLAUDE.md §5.3 step 1, §8.5).
//!
//! [`inspect`] reads the format and pixel size from the file's header without decoding
//! it, so a file that isn't a JPEG, PNG or WebP, or that would decode to too many pixels,
//! is refused before it costs any memory. The API can call it at upload time (M1-D4).
//! [`decode`] inspects, then decodes to BGR, turning the photo upright by its EXIF
//! orientation.
//!
//! The browser sends photos already upright and re-encoded through a canvas (M1-E2), so
//! most have no EXIF left; a photo sent straight from a phone is still turned correctly.

use opencv::core::{Mat, Vector};
use opencv::imgcodecs;
use opencv::prelude::*;
use serde::{Deserialize, Serialize};

use crate::error::VisionError;

/// At most this many pixels: room for a 8000 × 5000 photo. The browser sends at most
/// 4000 px on the long side (M1-E2); a decoded image takes 3 bytes per pixel.
pub const MAX_PIXELS: u64 = 40_000_000;
/// The long side needs at least this many pixels: a sheet about 1000 px long gives about
/// 3 px per mm, the least that measures to the §5.6 targets.
pub const MIN_LONG_SIDE_PX: u32 = 1000;

/// The image formats accepted. HEIC isn't: the browser converts it to JPEG (M1-E2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageFormat {
    Jpeg,
    Png,
    Webp,
}

/// What a file's header says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageInfo {
    pub format: ImageFormat,
    /// As stored, before any EXIF turn.
    pub width: u32,
    pub height: u32,
}

/// The format and size from the file's header, checked against [`MAX_PIXELS`] and
/// [`MIN_LONG_SIDE_PX`]. Nothing is decoded.
pub fn inspect(bytes: &[u8]) -> Result<ImageInfo, VisionError> {
    let info = header(bytes)?;
    let (width, height) = (info.width, info.height);
    if width == 0 || height == 0 {
        return Err(VisionError::UnreadableImage);
    }
    if u64::from(width) * u64::from(height) > MAX_PIXELS {
        return Err(VisionError::ImageTooLarge {
            width,
            height,
            max_pixels: MAX_PIXELS,
        });
    }
    if width.max(height) < MIN_LONG_SIDE_PX {
        return Err(VisionError::ImageTooSmall {
            width,
            height,
            min_long_side: MIN_LONG_SIDE_PX,
        });
    }
    Ok(info)
}

/// The photo as BGR pixels, upright by its EXIF orientation, after [`inspect`] accepts it.
pub fn decode(bytes: &[u8]) -> Result<Mat, VisionError> {
    let info = inspect(bytes)?;
    let buffer = Vector::<u8>::from_slice(bytes);
    let image = imgcodecs::imdecode(&buffer, imgcodecs::IMREAD_COLOR)?;
    if image.empty() {
        return Err(VisionError::UnreadableImage);
    }
    // The pixels must be the ones the header promised, either way round.
    let decoded = (image.cols(), image.rows());
    let (width, height) = (info.width as i32, info.height as i32);
    if decoded != (width, height) && decoded != (height, width) {
        return Err(VisionError::UnreadableImage);
    }
    Ok(image)
}

/// The format and size, from the first bytes.
fn header(bytes: &[u8]) -> Result<ImageInfo, VisionError> {
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return jpeg(bytes);
    }
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return png(bytes);
    }
    if bytes.get(..4) == Some(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
        return webp(bytes);
    }
    Err(VisionError::UnsupportedImage)
}

/// PNG: the first chunk is IHDR, with the width and height big-endian.
fn png(bytes: &[u8]) -> Result<ImageInfo, VisionError> {
    if bytes.get(12..16) != Some(b"IHDR") {
        return Err(VisionError::UnreadableImage);
    }
    Ok(ImageInfo {
        format: ImageFormat::Png,
        width: be_u32((bytes, 16))?,
        height: be_u32((bytes, 20))?,
    })
}

/// JPEG: walks the segments to the first start-of-frame, which holds the height and
/// width.
fn jpeg(bytes: &[u8]) -> Result<ImageInfo, VisionError> {
    let mut at = 2;
    loop {
        if byte((bytes, at))? != 0xFF {
            return Err(VisionError::UnreadableImage);
        }
        // Any number of 0xFF fill bytes may come before the marker.
        while byte((bytes, at + 1))? == 0xFF {
            at += 1;
        }
        let marker = byte((bytes, at + 1))?;
        at += 2;
        match marker {
            // Markers that stand alone, with no length.
            0x01 | 0xD0..=0xD7 => continue,
            // Image data or the end before any frame: there's no size to read.
            0xD9 | 0xDA => return Err(VisionError::UnreadableImage),
            // Start of frame (every kind but the DHT, JPG and DAC markers among them).
            0xC0..=0xCF if !matches!(marker, 0xC4 | 0xC8 | 0xCC) => {
                return Ok(ImageInfo {
                    format: ImageFormat::Jpeg,
                    height: u32::from(be_u16((bytes, at + 3))?),
                    width: u32::from(be_u16((bytes, at + 5))?),
                });
            }
            _ => {
                let length = usize::from(be_u16((bytes, at))?);
                if length < 2 {
                    return Err(VisionError::UnreadableImage);
                }
                at += length;
            }
        }
    }
}

/// WebP: the first chunk after the RIFF header is lossy (`VP8 `), lossless (`VP8L`) or
/// extended (`VP8X`), each keeping the size its own way.
fn webp(bytes: &[u8]) -> Result<ImageInfo, VisionError> {
    let data = 20;
    let (width, height) = match bytes.get(12..16) {
        // A key frame's tag (3 bytes) and start code (9D 01 2A), then 14-bit sizes.
        Some(b"VP8 ") => {
            if bytes.get(data + 3..data + 6) != Some(&[0x9D, 0x01, 0x2A]) {
                return Err(VisionError::UnreadableImage);
            }
            (
                u32::from(le_u16((bytes, data + 6))? & 0x3FFF),
                u32::from(le_u16((bytes, data + 8))? & 0x3FFF),
            )
        }
        // A signature byte (2F), then the width and height less one, 14 bits each.
        Some(b"VP8L") => {
            if byte((bytes, data))? != 0x2F {
                return Err(VisionError::UnreadableImage);
            }
            let bits = le_u32((bytes, data + 1))?;
            ((bits & 0x3FFF) + 1, ((bits >> 14) & 0x3FFF) + 1)
        }
        // Flags (1 byte) and 3 reserved, then the canvas width and height less one, 24
        // bits each.
        Some(b"VP8X") => (
            le_u24((bytes, data + 4))? + 1,
            le_u24((bytes, data + 7))? + 1,
        ),
        _ => return Err(VisionError::UnreadableImage),
    };
    Ok(ImageInfo {
        format: ImageFormat::Webp,
        width,
        height,
    })
}

fn byte((bytes, at): (&[u8], usize)) -> Result<u8, VisionError> {
    bytes.get(at).copied().ok_or(VisionError::UnreadableImage)
}

fn take<const N: usize>((bytes, at): (&[u8], usize)) -> Result<[u8; N], VisionError> {
    bytes
        .get(at..at + N)
        .and_then(|slice| slice.try_into().ok())
        .ok_or(VisionError::UnreadableImage)
}

fn be_u16(at: (&[u8], usize)) -> Result<u16, VisionError> {
    take::<2>(at).map(u16::from_be_bytes)
}

fn be_u32(at: (&[u8], usize)) -> Result<u32, VisionError> {
    take::<4>(at).map(u32::from_be_bytes)
}

fn le_u16(at: (&[u8], usize)) -> Result<u16, VisionError> {
    take::<2>(at).map(u16::from_le_bytes)
}

fn le_u32(at: (&[u8], usize)) -> Result<u32, VisionError> {
    take::<4>(at).map(u32::from_le_bytes)
}

fn le_u24(at: (&[u8], usize)) -> Result<u32, VisionError> {
    take::<3>(at).map(|[low, middle, high]| u32::from_le_bytes([low, middle, high, 0]))
}
