//! Uploaded photos as files (CLAUDE.md §8.5): what kind of image a file is, how big, and a
//! copy of it without the metadata. Pure byte work: nothing is decoded, so the API checks and
//! cleans an upload in the request without OpenCV (M1-D4), and `vision` reads the same
//! header before decoding (M1-C1).
//!
//! - [`inspect`] reads the format and pixel size from the header and refuses anything that
//!   isn't a JPEG, PNG or WebP, or that would decode to too many or too few pixels.
//! - [`clean`] inspects, then copies the file keeping only what shows the picture: the image
//!   data, its colour profile and its EXIF orientation. GPS and every other EXIF field, XMP,
//!   IPTC, comments, thumbnails and anything after the image (a phone's depth or gain map)
//!   are left out. The pixels are the same bytes, not re-encoded.

mod bytes;
mod exif;
mod jpeg;
mod png;
mod webp;

use serde::{Deserialize, Serialize};
use thiserror::Error;

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

impl ImageFormat {
    /// The file extension, without the dot: `jpg`, `png`, `webp`.
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Jpeg => "jpg",
            Self::Png => "png",
            Self::Webp => "webp",
        }
    }

    /// The media type, e.g. `image/jpeg`.
    pub const fn media_type(self) -> &'static str {
        match self {
            Self::Jpeg => "image/jpeg",
            Self::Png => "image/png",
            Self::Webp => "image/webp",
        }
    }
}

/// What a file's header says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageInfo {
    pub format: ImageFormat,
    /// As stored, before any EXIF turn.
    pub width: u32,
    pub height: u32,
}

/// A photo ready to keep: the same picture, with nothing else but its colour profile and
/// orientation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanPhoto {
    /// The file.
    pub bytes: Vec<u8>,
    /// Its format and size (the same as the upload's).
    pub info: ImageInfo,
}

/// Why a file isn't accepted as a photo. The codes match `vision`'s for the same problems.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PhotoError {
    /// The file isn't a JPEG, PNG or WebP image.
    #[error("not a JPEG, PNG or WebP image: send the photo as a JPEG")]
    UnsupportedImage,
    /// The file says it's an image, but it can't be read.
    #[error("the image can't be read: it may be damaged or cut short")]
    UnreadableImage,
    /// More pixels than a photo needs: refused before decoding, so a crafted file can't
    /// eat the server's memory (CLAUDE.md §8.5).
    #[error("the image is {width} x {height} px; at most {max_pixels} pixels are accepted")]
    ImageTooLarge {
        width: u32,
        height: u32,
        max_pixels: u64,
    },
    /// Too few pixels to measure to a millimetre.
    #[error("the image is {width} x {height} px; its long side needs at least {min_long_side} px")]
    ImageTooSmall {
        width: u32,
        height: u32,
        min_long_side: u32,
    },
}

impl PhotoError {
    /// The stable error code the API sends and translates, e.g. `CAPTURE_IMAGE_TOO_SMALL`.
    pub const fn code(&self) -> &'static str {
        match self {
            Self::UnsupportedImage => "CAPTURE_UNSUPPORTED_IMAGE",
            Self::UnreadableImage => "CAPTURE_UNREADABLE_IMAGE",
            Self::ImageTooLarge { .. } => "CAPTURE_IMAGE_TOO_LARGE",
            Self::ImageTooSmall { .. } => "CAPTURE_IMAGE_TOO_SMALL",
        }
    }
}

/// The format and size from the file's header, checked against [`MAX_PIXELS`] and
/// [`MIN_LONG_SIDE_PX`]. Nothing is decoded.
pub fn inspect(bytes: &[u8]) -> Result<ImageInfo, PhotoError> {
    let info = header(bytes)?;
    let (width, height) = (info.width, info.height);
    if width == 0 || height == 0 {
        return Err(PhotoError::UnreadableImage);
    }
    if u64::from(width) * u64::from(height) > MAX_PIXELS {
        return Err(PhotoError::ImageTooLarge {
            width,
            height,
            max_pixels: MAX_PIXELS,
        });
    }
    if width.max(height) < MIN_LONG_SIDE_PX {
        return Err(PhotoError::ImageTooSmall {
            width,
            height,
            min_long_side: MIN_LONG_SIDE_PX,
        });
    }
    Ok(info)
}

/// [`inspect`]s the file, then copies it without its metadata: only the image data, its
/// colour profile and its EXIF orientation are kept (the orientation in a fresh EXIF block
/// holding nothing else). A file that is cut short or whose structure doesn't hold together
/// is `UnreadableImage`.
pub fn clean(bytes: &[u8]) -> Result<CleanPhoto, PhotoError> {
    let info = inspect(bytes)?;
    let bytes = match info.format {
        ImageFormat::Jpeg => jpeg::clean(bytes)?,
        ImageFormat::Png => png::clean(bytes)?,
        ImageFormat::Webp => webp::clean(bytes)?,
    };
    Ok(CleanPhoto { bytes, info })
}

/// The format and size, from the first bytes.
fn header(bytes: &[u8]) -> Result<ImageInfo, PhotoError> {
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return jpeg::size(bytes);
    }
    if bytes.starts_with(png::SIGNATURE) {
        return png::size(bytes);
    }
    if bytes.get(..4) == Some(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
        return webp::size(bytes);
    }
    Err(PhotoError::UnsupportedImage)
}
