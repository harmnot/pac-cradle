//! Captures: photo uploads (M1-D4), vision runs, reading results and corrections (M1-D5).

mod capture_service;
mod capture_service_trait;

pub use capture_service::{
    CaptureProviders, DefaultCaptureService, MAX_PHOTO_BYTES, MAX_PHOTO_MB, PHOTO_URL_TTL,
    VISION_AT_ONCE, file_too_large,
};
pub use capture_service_trait::{CaptureService, CorrectionRequest, PhotoUpload};
