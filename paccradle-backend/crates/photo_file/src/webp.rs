//! WebP: a RIFF file of chunks, each a four-letter code, a 4-byte little-endian size, the data
//! and a pad byte when the size is odd. A simple file holds one image chunk (`VP8 ` lossy or
//! `VP8L` lossless); an extended one starts with `VP8X`, whose flags say which optional chunks
//! follow (colour profile, alpha, animation, EXIF, XMP).

use crate::bytes::{byte, index, le_u16, le_u24, le_u32, slice};
use crate::exif;
use crate::{ImageFormat, ImageInfo, PhotoError};

/// Where the first chunk starts: after `RIFF`, the size and `WEBP`.
const FIRST_CHUNK: usize = 12;

/// The chunks that show the picture in an extended file. `EXIF`, `XMP ` and unknown chunks
/// are left out.
const KEPT: [&[u8; 4]; 7] = [
    b"VP8X", b"ICCP", b"ANIM", b"ANMF", b"ALPH", b"VP8 ", b"VP8L",
];

/// `VP8X` flags: an EXIF chunk and an XMP chunk follow.
const EXIF_FLAG: u8 = 0x08;
const XMP_FLAG: u8 = 0x04;

/// The size: the first chunk after the RIFF header is lossy (`VP8 `), lossless (`VP8L`) or
/// extended (`VP8X`), each keeping it its own way.
pub(crate) fn size(bytes: &[u8]) -> Result<ImageInfo, PhotoError> {
    let data = FIRST_CHUNK + 8;
    let (width, height) = match bytes.get(12..16) {
        // A key frame's tag (3 bytes) and start code (9D 01 2A), then 14-bit sizes.
        Some(b"VP8 ") => {
            if bytes.get(data + 3..data + 6) != Some(&[0x9D, 0x01, 0x2A]) {
                return Err(PhotoError::UnreadableImage);
            }
            (
                u32::from(le_u16((bytes, data + 6))? & 0x3FFF),
                u32::from(le_u16((bytes, data + 8))? & 0x3FFF),
            )
        }
        // A signature byte (2F), then the width and height less one, 14 bits each.
        Some(b"VP8L") => {
            if byte((bytes, data))? != 0x2F {
                return Err(PhotoError::UnreadableImage);
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
        _ => return Err(PhotoError::UnreadableImage),
    };
    Ok(ImageInfo {
        format: ImageFormat::Webp,
        width,
        height,
    })
}

/// A simple file keeps its one image chunk. An extended file keeps the [`KEPT`] chunks, with
/// the EXIF and XMP flags cleared; an orientation other than 1 comes back as a fresh `EXIF`
/// chunk holding only that, at the end, with its flag set. Anything past the RIFF size is
/// left out.
pub(crate) fn clean(bytes: &[u8]) -> Result<Vec<u8>, PhotoError> {
    let riff_end = 8 + index(le_u32((bytes, 4))?)?;
    let riff = slice(bytes, (0, riff_end))?;
    let chunks = chunks(riff)?;
    let &(first, first_chunk) = chunks.first().ok_or(PhotoError::UnreadableImage)?;
    if first != b"VP8X" {
        return Ok(riff_file(first_chunk.to_vec()));
    }
    // VP8X holds 10 bytes: the flags, 3 reserved, then the canvas size.
    if first_chunk.len() < 8 + 10 {
        return Err(PhotoError::UnreadableImage);
    }
    let mut body = Vec::with_capacity(riff.len());
    let mut orientation = None;
    for &(kind, chunk) in &chunks {
        if kind == b"EXIF" && orientation.is_none() {
            orientation = exif::orientation(chunk.get(8..).unwrap_or_default());
        }
        if KEPT.contains(&kind) {
            body.extend(chunk);
        }
    }
    let orientation = exif::turned(orientation);
    // The flags are the first data byte of VP8X, the first chunk written.
    let flags = body.get_mut(8).ok_or(PhotoError::UnreadableImage)?;
    *flags &= !(EXIF_FLAG | XMP_FLAG);
    if let Some(orientation) = orientation {
        *flags |= EXIF_FLAG;
        body.extend(chunk(b"EXIF", &exif::orientation_tiff(orientation)));
    }
    Ok(riff_file(body))
}

/// A chunk's four-letter code and its whole bytes (header, data and pad).
type Chunk<'a> = (&'a [u8; 4], &'a [u8]);

/// Every chunk. The last one may lack its pad byte.
fn chunks(riff: &[u8]) -> Result<Vec<Chunk<'_>>, PhotoError> {
    let mut chunks = Vec::new();
    let mut at = FIRST_CHUNK;
    while at < riff.len() {
        let kind = riff
            .get(at..at + 4)
            .and_then(|code| <&[u8; 4]>::try_from(code).ok())
            .ok_or(PhotoError::UnreadableImage)?;
        let size = index(le_u32((riff, at + 4))?)?;
        let data_end = (at + 8)
            .checked_add(size)
            .ok_or(PhotoError::UnreadableImage)?;
        if data_end > riff.len() {
            return Err(PhotoError::UnreadableImage);
        }
        let end = (data_end + size % 2).min(riff.len());
        chunks.push((kind, &riff[at..end]));
        at = end;
    }
    Ok(chunks)
}

/// A chunk of `kind` holding `data`, padded to an even length.
fn chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(9 + data.len());
    out.extend(kind);
    // Only ever a few dozen bytes here.
    out.extend((data.len() as u32).to_le_bytes());
    out.extend(data);
    if data.len() % 2 == 1 {
        out.push(0);
    }
    out
}

/// The RIFF header around `body`, its size counting `WEBP` and every chunk.
fn riff_file(body: Vec<u8>) -> Vec<u8> {
    let mut out = Vec::with_capacity(FIRST_CHUNK + body.len() + 1);
    out.extend(b"RIFF");
    // At most the upload's own size, which fit in 32 bits.
    out.extend(((4 + body.len() + body.len() % 2) as u32).to_le_bytes());
    out.extend(b"WEBP");
    out.extend(&body);
    // A last chunk that came without its pad byte gets one.
    if body.len() % 2 == 1 {
        out.push(0);
    }
    out
}
