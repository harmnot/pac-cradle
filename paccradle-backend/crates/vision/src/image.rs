//! Reading an uploaded photo safely (CLAUDE.md §5.3 step 1, §8.5).
//!
//! [`inspect`] reads the format and pixel size from the file's header without decoding
//! it, so a file that isn't a JPEG, PNG or WebP, or that would decode to too many pixels,
//! is refused before it costs any memory. The header code lives in `photo_file`, which the
//! API also calls at upload time (M1-D4). [`decode`] inspects, then decodes to BGR, turning
//! the photo upright by its EXIF orientation.
//!
//! The browser sends photos already upright and re-encoded through a canvas (M1-E2), so
//! most have no EXIF left; a photo sent straight from a phone is still turned correctly, and
//! the API keeps the orientation when it cleans the upload's metadata away.

use opencv::core::{Mat, Vector};
use opencv::imgcodecs;
use opencv::prelude::*;
pub use photo_file::{ImageFormat, ImageInfo, MAX_PIXELS, MIN_LONG_SIDE_PX};

use crate::error::VisionError;

/// The format and size from the file's header, checked against [`MAX_PIXELS`] and
/// [`MIN_LONG_SIDE_PX`]. Nothing is decoded.
pub fn inspect(bytes: &[u8]) -> Result<ImageInfo, VisionError> {
    Ok(photo_file::inspect(bytes)?)
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
