use axum::extract::multipart::{Field, MultipartError, MultipartRejection};
use axum::extract::{Multipart, Path, State};
use axum::http::StatusCode;
use axum::{Extension, Json};
use uuid::Uuid;

use crate::AppState;
use crate::error::{AppError, ErrorCode};
use crate::locales::Locale;
use crate::models::capture::Corrected;
use crate::models::session::AuthUser;
use crate::schemas::capture::{
    CaptureCreatedResponse, CaptureResponse, CorrectCaptureRequest, CreateCaptureRequest,
};
use crate::service::capture::{CorrectionRequest, MAX_PHOTO_BYTES, PhotoUpload, file_too_large};

/// The `capture` part is a few hundred bytes of JSON.
const MAX_SETTINGS_BYTES: usize = 16 * 1024;

/// `POST /captures` (`multipart/form-data`): a `photo` part (the image file: JPEG, PNG or
/// WebP, at most 15 MB) and a `capture` part (JSON settings, [`CreateCaptureRequest`]), in
/// either order. 201 with the pending capture; its first vision run is queued. Guests may
/// upload.
pub async fn create(
    State(state): State<AppState>,
    user: AuthUser,
    multipart: Result<Multipart, MultipartRejection>,
) -> Result<(StatusCode, Json<CaptureCreatedResponse>), AppError> {
    let mut multipart = multipart.map_err(|rejection| {
        AppError::new(ErrorCode::UnsupportedMediaType)
            .with_meta("expected", "multipart/form-data")
            .with_source(rejection)
    })?;
    let (settings, photo) = read_parts(&mut multipart).await?;
    let (settings, exif) = CreateCaptureRequest::from_json(&settings)?.into_parts()?;
    let created = state
        .captures
        .create(PhotoUpload {
            user,
            settings,
            exif,
            photo,
        })
        .await?;
    Ok((StatusCode::CREATED, Json(created.into())))
}

/// `GET /captures/{capture_id}`: the caller's capture, its outline once measured, and a
/// 10-minute link to its photo. Poll it while `status` is `pending`.
pub async fn get(
    State(state): State<AppState>,
    Extension(locale): Extension<Locale>,
    user: AuthUser,
    Path(capture_id): Path<Uuid>,
) -> Result<Json<CaptureResponse>, AppError> {
    let view = state.captures.get((&user, capture_id)).await?;
    Ok(Json(CaptureResponse::new((view, locale))))
}

/// `POST /captures/{capture_id}/correct`: a tap (202, a new run is pending), an edited
/// outline or a height (200, applied at once; a height typed while a run is pending queues a
/// new one, 202).
pub async fn correct(
    State(state): State<AppState>,
    Extension(locale): Extension<Locale>,
    user: AuthUser,
    Path(capture_id): Path<Uuid>,
    Json(body): Json<CorrectCaptureRequest>,
) -> Result<(StatusCode, Json<CaptureResponse>), AppError> {
    let request = CorrectionRequest {
        user: &user,
        capture_id,
        correction: body.correction()?,
    };
    let (view, corrected) = state.captures.correct(request).await?;
    let status = match corrected {
        Corrected::Done => StatusCode::OK,
        Corrected::Running => StatusCode::ACCEPTED,
    };
    Ok((status, Json(CaptureResponse::new((view, locale)))))
}

/// The `capture` and `photo` parts' bytes. A missing, repeated or unknown part is
/// `INVALID_REQUEST` naming it.
async fn read_parts(multipart: &mut Multipart) -> Result<(Vec<u8>, Vec<u8>), AppError> {
    let mut settings = None;
    let mut photo = None;
    while let Some(field) = multipart.next_field().await.map_err(multipart_error)? {
        let name = field.name().unwrap_or_default().to_owned();
        match name.as_str() {
            "capture" if settings.is_none() => {
                let bytes = read_field((field, MAX_SETTINGS_BYTES))
                    .await?
                    .ok_or_else(|| invalid_part("capture"))?;
                settings = Some(bytes);
            }
            "photo" if photo.is_none() => {
                let bytes = read_field((field, MAX_PHOTO_BYTES))
                    .await?
                    .ok_or_else(file_too_large)?;
                photo = Some(bytes);
            }
            _ => return Err(invalid_part(&name)),
        }
    }
    let settings = settings.ok_or_else(|| invalid_part("capture"))?;
    let photo = photo.ok_or_else(|| invalid_part("photo"))?;
    Ok((settings, photo))
}

/// A part's bytes, or `None` once they pass `limit`.
async fn read_field((mut field, limit): (Field<'_>, usize)) -> Result<Option<Vec<u8>>, AppError> {
    let mut bytes = Vec::new();
    while let Some(chunk) = field.chunk().await.map_err(multipart_error)? {
        if bytes.len() + chunk.len() > limit {
            return Ok(None);
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(Some(bytes))
}

/// The body ran past the route's limit (`CAPTURE_FILE_TOO_LARGE`), or isn't valid multipart.
fn multipart_error(error: MultipartError) -> AppError {
    if error.status() == StatusCode::PAYLOAD_TOO_LARGE {
        return file_too_large();
    }
    AppError::new(ErrorCode::InvalidRequest)
        .with_meta("field", "multipart")
        .with_source(error)
}

fn invalid_part(name: &str) -> AppError {
    AppError::new(ErrorCode::InvalidRequest).with_meta("field", name.to_owned())
}
