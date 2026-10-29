//! Errors from the vision pipeline.
//!
//! Each error has a stable [`code`](VisionError::code) the API turns into a localized
//! message (`src/locales`); the English `Display` text is for logs and the CLI tools.

use thiserror::Error;

/// Why a photo couldn't be measured.
#[derive(Debug, Error)]
pub enum VisionError {
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
    /// A setting outside what the pipeline accepts.
    #[error("`{field}` is {value}; allowed: {allowed}")]
    InvalidInput {
        field: &'static str,
        value: String,
        allowed: &'static str,
    },
    /// No A4/Letter-shaped sheet was found.
    #[error(
        "no paper found: put the paper on a darker surface and keep all 4 corners in the photo"
    )]
    PaperNotFound,
    /// The paper was found but nothing on it stands out.
    #[error("no product found on the paper: put it in the middle, at least 2 cm from the edges")]
    ProductNotFound,
    /// Trace mode found no closed pen line.
    #[error(
        "no closed pen outline found: trace all the way around the product, then remove it before the photo"
    )]
    TraceNotFound,
    /// An OpenCV call failed.
    #[error("image processing failed: {0}")]
    OpenCv(#[from] opencv::Error),
}

impl VisionError {
    /// The stable error code the API sends and translates, e.g.
    /// `CAPTURE_PAPER_NOT_FOUND`.
    pub fn code(&self) -> &'static str {
        match self {
            VisionError::UnsupportedImage => "CAPTURE_UNSUPPORTED_IMAGE",
            VisionError::UnreadableImage => "CAPTURE_UNREADABLE_IMAGE",
            VisionError::ImageTooLarge { .. } => "CAPTURE_IMAGE_TOO_LARGE",
            VisionError::ImageTooSmall { .. } => "CAPTURE_IMAGE_TOO_SMALL",
            VisionError::InvalidInput { .. } => "CAPTURE_INVALID_INPUT",
            VisionError::PaperNotFound => "CAPTURE_PAPER_NOT_FOUND",
            VisionError::ProductNotFound => "CAPTURE_PRODUCT_NOT_FOUND",
            VisionError::TraceNotFound => "CAPTURE_TRACE_NOT_FOUND",
            VisionError::OpenCv(_) => "CAPTURE_PROCESSING_FAILED",
        }
    }

    /// True when the user can fix it with another photo, file or setting; false for an
    /// internal failure, which the API logs and doesn't explain.
    pub fn is_users_to_fix(&self) -> bool {
        !matches!(self, VisionError::OpenCv(_))
    }
}
