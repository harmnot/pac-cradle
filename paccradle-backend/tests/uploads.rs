//! Photo uploads (M1-D4), end to end on a real database with photos kept in memory:
//! `POST /captures` checks the file from its header, keeps it without its metadata, stores it
//! for 30 days and adds a pending capture; limits, bad parts and outages leave nothing behind.
// The helpers below are test code too; clippy.toml only exempts `#[test]` functions.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use axum::http::{Method, StatusCode};
use common::{
    CAPTURES, Part, Reply, TestApi, WEB_ORIGIN, auth_api, call, photo_part, settings_part, upload,
};
use paccradle_backend::handlers::client::capture::capture_route::UPLOAD_BODY_LIMIT;
use paccradle_backend::service::capture::MAX_PHOTO_BYTES;
use serde_json::{Value, json};
use sqlx::PgPool;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use uuid::Uuid;

/// A JPEG segment.
fn segment(marker: u8, payload: &[u8]) -> Vec<u8> {
    let mut segment = vec![0xFF, marker];
    segment.extend(u16::try_from(payload.len() + 2).unwrap().to_be_bytes());
    segment.extend(payload);
    segment
}

/// What a phone sends: EXIF with the orientation (6), the camera and a GPS position, a
/// comment, then a 4000 × 3000 picture (tables, frame, one scan) and a gain map after it.
fn phone_jpeg() -> Vec<u8> {
    let mut tiff = b"II*\0\x08\0\0\0\x03\0".to_vec();
    // Orientation 6; the camera's name at 50; GPS directory at 64.
    tiff.extend(b"\x12\x01\x03\0\x01\0\0\0\x06\0\0\0");
    tiff.extend(b"\x0F\x01\x02\0\x0E\0\0\0\x32\0\0\0");
    tiff.extend(b"\x25\x88\x04\0\x01\0\0\0\x40\0\0\0");
    tiff.extend(0_u32.to_le_bytes());
    tiff.extend(b"SECRET-PHONE\0\0");
    tiff.extend(b"\x01\0\x02\0\x05\0\x03\0\0\0\x52\0\0\0");
    tiff.extend(0_u32.to_le_bytes());
    tiff.extend(b"SECRET-LATITUDE-SECRET!!");
    let mut frame = vec![8];
    frame.extend(3000_u16.to_be_bytes());
    frame.extend(4000_u16.to_be_bytes());
    frame.extend([3, 1, 0x11, 0, 2, 0x11, 0, 3, 0x11, 0]);
    [
        vec![0xFF, 0xD8],
        segment(0xE1, &[b"Exif\0\0".as_slice(), &tiff].concat()),
        segment(0xFE, b"SECRET-COMMENT"),
        segment(0xDB, &[0; 65]),
        segment(0xC0, &frame),
        segment(0xC4, &[0, 1, 2, 3]),
        segment(0xDA, &[3, 1, 0, 2, 0x11, 3, 0x11, 0, 63, 0]),
        b"\x12\x34\xFF\x00\x56".to_vec(),
        vec![0xFF, 0xD9],
        b"\xFF\xD8SECRET-GAINMAP\xFF\xD9".to_vec(),
    ]
    .concat()
}

fn photo_settings() -> Value {
    json!({
        "paper": "a4",
        "mode": "photo",
        "height_mm": 95,
        "exif": {
            "make": " Apple ",
            "model": "iPhone 15 Pro",
            "focal_length_35mm": 26,
            "GPSLatitude": [51, 30, 26.4],
            "gps": { "latitude": 51.5 }
        }
    })
}

/// A new guest's access token.
async fn guest(api: &TestApi) -> String {
    let reply = api
        .send(call(Method::POST, "/api/client/v1/auth/guest").origin(WEB_ORIGIN))
        .await;
    assert_eq!(reply.status, StatusCode::CREATED);
    reply.access_token()
}

async fn capture_count(pool: &PgPool) -> i64 {
    sqlx::query_scalar!(r#"SELECT count(*) AS "count!" FROM captures"#)
        .fetch_one(pool)
        .await
        .unwrap()
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

#[sqlx::test]
async fn a_guest_uploads_a_photo_and_it_is_kept_without_its_metadata(pool: PgPool) {
    let api = auth_api(pool, &[]);
    let token = guest(&api).await;
    let original = phone_jpeg();

    let reply = upload(
        &api,
        (
            &token,
            &[
                settings_part(&photo_settings()),
                photo_part(original.clone()),
            ],
        ),
    )
    .await;

    assert_eq!(reply.status, StatusCode::CREATED, "{:?}", reply.body);
    assert_eq!(reply.header("api-version"), "1.0.0-beta.7");
    let body = reply.json();
    let id: Uuid = body["capture_id"].as_str().unwrap().parse().unwrap();
    assert_eq!(body["status"], "pending");
    assert_eq!(body["mode"], "photo");
    assert_eq!(body["paper"], "a4");
    assert_eq!(
        body["photo"],
        json!({ "format": "jpeg", "width": 4000, "height": 3000 })
    );

    // Stored as the same picture with its orientation, and nothing else.
    let key = format!("photos/{id}.jpg");
    assert_eq!(api.storage.keys(), vec![key.clone()]);
    let (stored, content_type) = api.storage.object(&key).unwrap();
    assert_eq!(content_type, "image/jpeg");
    assert_eq!(stored, photo_file::clean(&original).unwrap().bytes);
    assert!(!contains(&stored, b"SECRET"));
    assert!(contains(&stored, b"Exif\0\0MM\0*"));

    // The row: pending, vision's settings as `vision::CaptureInput` reads them, the camera
    // fields and no location, and the photo kept for 30 days.
    let row = sqlx::query!(
        r#"
        SELECT status, mode, paper, input, exif, storage_key, result,
               extract(epoch FROM purge_after - created_at)::float8 AS "kept_seconds!"
        FROM captures WHERE id = $1
        "#,
        id
    )
    .fetch_one(&api.pool)
    .await
    .unwrap();
    assert_eq!(
        (row.status.as_str(), row.mode.as_str(), row.paper.as_str()),
        ("pending", "photo", "a4")
    );
    assert_eq!(
        row.input,
        json!({
            "paper": "a4",
            "mode": "photo",
            "pen_width_mm": 0.5,
            "height_mm": 95.0,
            "focal_length_35mm": 26.0
        })
    );
    assert_eq!(
        row.exif,
        json!({ "make": "Apple", "model": "iPhone 15 Pro", "focal_length_35mm": 26.0 })
    );
    assert_eq!(row.storage_key.as_deref(), Some(key.as_str()));
    // Its first vision run is queued with it (M1-D5).
    let jobs = sqlx::query_scalar!("SELECT payload FROM jobs WHERE kind = 'run_vision'")
        .fetch_all(&api.pool)
        .await
        .unwrap();
    assert_eq!(jobs, [json!({ "capture_id": id, "run": 1 })]);
    assert_eq!(row.result, None);
    assert_eq!(row.kept_seconds, 30.0 * 24.0 * 60.0 * 60.0);
    let time = |name: &str| {
        OffsetDateTime::parse(body[name].as_str().unwrap(), &Rfc3339).expect("RFC 3339")
    };
    assert_eq!(
        time("photo_kept_until") - time("created_at"),
        time::Duration::days(30)
    );
}

#[sqlx::test]
async fn a_trace_keeps_its_pen_width_and_the_parts_may_come_in_any_order(pool: PgPool) {
    let api = auth_api(pool, &[]);
    let token = guest(&api).await;
    let settings = json!({ "paper": "letter", "mode": "trace", "pen_width_mm": 1.2 });

    let reply = upload(
        &api,
        (
            &token,
            &[photo_part(phone_jpeg()), settings_part(&settings)],
        ),
    )
    .await;

    assert_eq!(reply.status, StatusCode::CREATED, "{:?}", reply.body);
    let input = sqlx::query_scalar!("SELECT input FROM captures")
        .fetch_one(&api.pool)
        .await
        .unwrap();
    assert_eq!(
        input,
        json!({ "paper": "letter", "mode": "trace", "pen_width_mm": 1.2 })
    );
    let exif = sqlx::query_scalar!("SELECT exif FROM captures")
        .fetch_one(&api.pool)
        .await
        .unwrap();
    assert_eq!(exif, json!({}));
}

#[sqlx::test]
async fn uploads_need_a_session(pool: PgPool) {
    let api = auth_api(pool, &[]);

    let reply = upload(
        &api,
        (
            "not-a-token",
            &[settings_part(&photo_settings()), photo_part(phone_jpeg())],
        ),
    )
    .await;

    assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
    assert_eq!(reply.error_code(), "AUTH_REQUIRED");
    assert!(api.storage.keys().is_empty());
}

#[sqlx::test]
async fn the_parts_are_checked(pool: PgPool) {
    let api = auth_api(pool, &[]);
    let token = guest(&api).await;
    let field = |reply: &Reply| reply.json()["error"]["meta"]["field"].clone();

    let cases: Vec<(Vec<Part<'_>>, &str)> = vec![
        (vec![settings_part(&photo_settings())], "photo"),
        (vec![photo_part(phone_jpeg())], "capture"),
        (
            vec![
                settings_part(&photo_settings()),
                photo_part(phone_jpeg()),
                photo_part(phone_jpeg()),
            ],
            "photo",
        ),
        (
            vec![
                settings_part(&photo_settings()),
                photo_part(phone_jpeg()),
                Part {
                    name: "extra",
                    file_name: None,
                    bytes: b"1".to_vec(),
                },
            ],
            "extra",
        ),
        (
            vec![
                Part {
                    name: "capture",
                    file_name: None,
                    bytes: b"{ not json".to_vec(),
                },
                photo_part(phone_jpeg()),
            ],
            "capture",
        ),
        // A misspelt setting isn't silently ignored.
        (
            vec![
                settings_part(&json!({ "paper": "a4", "mode": "photo", "height": 95 })),
                photo_part(phone_jpeg()),
            ],
            "capture",
        ),
        (
            vec![
                settings_part(&json!({ "paper": "a3", "mode": "photo" })),
                photo_part(phone_jpeg()),
            ],
            "capture",
        ),
    ];
    for (parts, expected) in cases {
        let reply = upload(&api, (&token, &parts)).await;
        assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{expected}");
        assert_eq!(reply.error_code(), "INVALID_REQUEST");
        assert_eq!(field(&reply), expected);
    }

    // Not multipart at all.
    let reply = api
        .send(
            call(Method::POST, CAPTURES)
                .bearer(&token)
                .json(photo_settings()),
        )
        .await;
    assert_eq!(reply.status, StatusCode::UNSUPPORTED_MEDIA_TYPE);
    assert_eq!(reply.error_code(), "UNSUPPORTED_MEDIA_TYPE");

    assert!(api.storage.keys().is_empty());
    assert_eq!(capture_count(&api.pool).await, 0);
}

#[sqlx::test]
async fn settings_out_of_range_name_their_field(pool: PgPool) {
    let api = auth_api(pool, &[]);
    let token = guest(&api).await;
    let cases = [
        (json!({ "pen_width_mm": 5 }), "pen_width_mm", "0.1 to 3 mm"),
        (json!({ "height_mm": 0 }), "height_mm", "0.1 to 1000 mm"),
        (
            json!({ "exif": { "focal_length_35mm": 2 } }),
            "exif.focal_length_35mm",
            "5 to 3000 mm",
        ),
        (
            json!({ "exif": { "model": "x".repeat(101) } }),
            "exif.model",
            "at most 100 characters",
        ),
    ];
    for (extra, field, allowed) in cases {
        let mut settings = json!({ "paper": "a4", "mode": "trace" });
        for (name, value) in extra.as_object().unwrap() {
            settings[name] = value.clone();
        }
        let reply = upload(
            &api,
            (
                &token,
                &[settings_part(&settings), photo_part(phone_jpeg())],
            ),
        )
        .await;
        assert_eq!(reply.status, StatusCode::BAD_REQUEST);
        assert_eq!(reply.error_code(), "CAPTURE_INVALID_INPUT");
        assert_eq!(reply.json()["error"]["meta"]["field"], field);
        assert_eq!(
            reply.json()["error"]["message"],
            format!("The setting {field} must be {allowed}.")
        );
    }
    assert!(api.storage.keys().is_empty());
}

/// A PNG of `width` × `height` with nothing but its header.
fn png_header((width, height): (u32, u32)) -> Vec<u8> {
    let mut png = b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR".to_vec();
    png.extend(width.to_be_bytes());
    png.extend(height.to_be_bytes());
    png.extend([8, 2, 0, 0, 0, 0, 0, 0, 0]);
    png
}

#[sqlx::test]
async fn files_that_are_not_photos_are_refused_before_anything_is_stored(pool: PgPool) {
    let api = auth_api(pool, &[]);
    let token = guest(&api).await;
    let mut cut_short = phone_jpeg();
    cut_short.truncate(cut_short.len() - 30);
    let cases = [
        (
            b"GIF89a\x01\0\x01\0".to_vec(),
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "CAPTURE_UNSUPPORTED_IMAGE",
        ),
        (
            png_header((8000, 5001)),
            StatusCode::PAYLOAD_TOO_LARGE,
            "CAPTURE_IMAGE_TOO_LARGE",
        ),
        (
            png_header((999, 750)),
            StatusCode::BAD_REQUEST,
            "CAPTURE_IMAGE_TOO_SMALL",
        ),
        (
            cut_short,
            StatusCode::BAD_REQUEST,
            "CAPTURE_UNREADABLE_IMAGE",
        ),
    ];
    for (file, status, code) in cases {
        let reply = upload(
            &api,
            (
                &token,
                &[settings_part(&photo_settings()), photo_part(file)],
            ),
        )
        .await;
        assert_eq!((reply.status, reply.error_code().as_str()), (status, code));
    }
    let reply = upload(
        &api,
        (
            &token,
            &[
                settings_part(&photo_settings()),
                photo_part(png_header((8000, 5001))),
            ],
        ),
    )
    .await;
    assert_eq!(
        reply.json()["error"]["message"],
        "The photo has too many pixels (8000 × 5001). Send one of at most 40 megapixels."
    );
    assert!(api.storage.keys().is_empty());
    assert_eq!(capture_count(&api.pool).await, 0);
}

#[sqlx::test]
async fn a_photo_over_15_mb_is_refused(pool: PgPool) {
    let api = auth_api(pool, &[]);
    let token = guest(&api).await;
    // Just over the photo's limit, within the body's; then far over the body's.
    for size in [MAX_PHOTO_BYTES + 1, UPLOAD_BODY_LIMIT + 1] {
        let mut photo = phone_jpeg();
        photo.resize(size, 0);
        let reply = upload(
            &api,
            (
                &token,
                &[settings_part(&photo_settings()), photo_part(photo)],
            ),
        )
        .await;
        assert_eq!(reply.status, StatusCode::PAYLOAD_TOO_LARGE, "{size}");
        assert_eq!(reply.error_code(), "CAPTURE_FILE_TOO_LARGE");
        assert_eq!(reply.json()["error"]["meta"]["max_mb"], 15);
    }
    assert!(api.storage.keys().is_empty());
}

#[sqlx::test]
async fn uploads_are_limited_per_user_and_per_address(pool: PgPool) {
    let api = auth_api(pool, &[("RATE_LIMIT_UPLOADS_PER_HOUR", "2")]);
    let parts = || [settings_part(&photo_settings()), photo_part(phone_jpeg())];

    // Two an hour for a user.
    let first = guest(&api).await;
    for _ in 0..2 {
        assert_eq!(
            upload(&api, (&first, &parts())).await.status,
            StatusCode::CREATED
        );
    }
    let reply = upload(&api, (&first, &parts())).await;
    assert_eq!(reply.status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(reply.error_code(), "RATE_LIMITED");
    assert!(
        reply.json()["error"]["meta"]["retry_after_seconds"]
            .as_u64()
            .unwrap()
            > 1000
    );

    // Four times that for an address (every test request comes from the same one): the
    // third request above counted too, so after five more the address is out.
    for _ in 0..5 {
        let other = guest(&api).await;
        assert_eq!(
            upload(&api, (&other, &parts())).await.status,
            StatusCode::CREATED
        );
    }
    let last = guest(&api).await;
    let reply = upload(&api, (&last, &parts())).await;
    assert_eq!(reply.status, StatusCode::TOO_MANY_REQUESTS);
    assert!(!reply.header("retry-after").is_empty());
    assert_eq!(capture_count(&api.pool).await, 7);
}

#[sqlx::test]
async fn when_storage_is_down_nothing_is_saved(pool: PgPool) {
    let api = auth_api(pool, &[]);
    let token = guest(&api).await;
    api.storage.set_available(false);

    let reply = upload(
        &api,
        (
            &token,
            &[settings_part(&photo_settings()), photo_part(phone_jpeg())],
        ),
    )
    .await;

    assert_eq!(reply.status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(reply.error_code(), "SERVICE_UNAVAILABLE");
    assert_eq!(reply.json()["error"]["meta"]["dependency"], "storage");
    assert_eq!(capture_count(&api.pool).await, 0);
}

#[sqlx::test]
async fn a_capture_that_cannot_be_saved_leaves_no_photo_behind(pool: PgPool) {
    let api = auth_api(pool, &[]);
    let token = guest(&api).await;
    // The guest goes while its access token still works: the row can't reference it.
    sqlx::query!("DELETE FROM users")
        .execute(&api.pool)
        .await
        .unwrap();

    let reply = upload(
        &api,
        (
            &token,
            &[settings_part(&photo_settings()), photo_part(phone_jpeg())],
        ),
    )
    .await;

    assert_eq!(reply.status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(api.storage.keys().is_empty());
}
