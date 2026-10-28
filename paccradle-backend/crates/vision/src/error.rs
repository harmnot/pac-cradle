//! Errors from the vision pipeline.

use thiserror::Error;

/// Why a photo couldn't be measured. Messages are written for the end user.
#[derive(Debug, Error)]
pub enum VisionError {
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
