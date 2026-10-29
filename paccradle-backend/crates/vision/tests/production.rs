//! M1-C1: the production entry point. [`vision::run`] measures an uploaded file in every
//! accepted format; files are checked from their header before decoding (format, too
//! many or too few pixels, broken); EXIF orientation turns a photo upright; the job's
//! input reads from JSON and is checked; the result has the documented §5.5 shape and
//! reads back from JSON; error codes are stable.
// The helpers below are test code too; clippy.toml only exempts `#[test]` functions.
#![allow(clippy::unwrap_used)]

mod common;

use std::error::Error;

use common::{Drawing, TOLERANCE_MM, UPRIGHT_QUAD, assert_close, encode, paper_image, photograph};
use opencv::core::{self, Mat, Scalar, Size, Vector};
use opencv::imgcodecs;
use opencv::prelude::*;
use serde_json::json;
use vision::{
    CaptureInput, CaptureMode, ImageFormat, ImageInfo, MAX_PIXELS, MIN_LONG_SIDE_PX, PaperSize,
    VisionError, VisionResult,
};

type TestResult = Result<(), Box<dyn Error>>;

/// A 52 mm disc on A4, photographed at a tilt (4000 × 3000 px).
fn disc_photo() -> opencv::Result<Mat> {
    let drawing = Drawing::Disc {
        center_mm: (105.0, 150.0),
        diameter_mm: 52.0,
    };
    photograph(&paper_image(PaperSize::A4, &drawing)?, UPRIGHT_QUAD)
}

fn plain((width, height): (i32, i32)) -> opencv::Result<Mat> {
    Mat::new_size_with_default(
        Size::new(width, height),
        core::CV_8UC3,
        Scalar::new(120.0, 140.0, 160.0, 0.0),
    )
}

fn photo_input() -> CaptureInput {
    CaptureInput {
        paper: PaperSize::A4,
        mode: CaptureMode::Photo,
        pen_width_mm: vision::DEFAULT_PEN_WIDTH_MM,
        tap_point_px: None,
        height_mm: None,
        focal_length_35mm: None,
    }
}

#[test]
fn run_measures_an_uploaded_photo_in_every_accepted_format() -> TestResult {
    let photo = disc_photo()?;
    for extension in [".jpg", ".png", ".webp"] {
        let bytes = encode((&photo, extension))?;
        let result = vision::run((&photo_input(), &bytes))?;
        let product = &result.products[0];
        assert_close(
            &format!("{extension} diameter"),
            product.diameter_mm.ok_or("no diameter")?,
            52.0,
        );
    }
    Ok(())
}

#[test]
fn inspect_reads_format_and_size_from_the_header() -> TestResult {
    let image = plain((1200, 900))?;
    for (extension, format) in [
        (".jpg", ImageFormat::Jpeg),
        (".png", ImageFormat::Png),
        (".webp", ImageFormat::Webp),
    ] {
        let bytes = encode((&image, extension))?;
        assert_eq!(
            vision::inspect(&bytes)?,
            ImageInfo {
                format,
                width: 1200,
                height: 900
            },
            "{extension}"
        );
    }
    // Lossless WebP keeps its size another way.
    let lossless = {
        let params = Vector::from_slice(&[imgcodecs::IMWRITE_WEBP_QUALITY, 101]);
        let mut bytes = Vector::<u8>::new();
        imgcodecs::imencode(".webp", &image, &mut bytes, &params)?;
        bytes.to_vec()
    };
    assert_eq!(&lossless[12..16], b"VP8L");
    assert_eq!(vision::inspect(&lossless)?.width, 1200);
    Ok(())
}

/// Just a file's header, claiming `width` × `height`, with no pixels after it.
fn png_header((width, height): (u32, u32)) -> Vec<u8> {
    let mut bytes = b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR".to_vec();
    bytes.extend(width.to_be_bytes());
    bytes.extend(height.to_be_bytes());
    bytes.extend([8, 2, 0, 0, 0, 0, 0, 0, 0]);
    bytes
}

fn jpeg_header((width, height): (u16, u16)) -> Vec<u8> {
    // Start of image, a JFIF segment to skip, then a baseline frame header.
    let mut bytes = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10];
    bytes.extend(b"JFIF\0\x01\x01\0\0\x01\0\x01\0\0");
    bytes.extend([0xFF, 0xC0, 0x00, 0x11, 0x08]);
    bytes.extend(height.to_be_bytes());
    bytes.extend(width.to_be_bytes());
    bytes.extend([0x03, 0x01, 0x22, 0x00, 0x02, 0x11, 0x01, 0x03, 0x11, 0x01]);
    bytes
}

fn webp_header((width, height): (u32, u32)) -> Vec<u8> {
    let mut bytes = b"RIFF\0\0\0\0WEBPVP8X\x0a\0\0\0\0\0\0\0".to_vec();
    bytes.extend(&(width - 1).to_le_bytes()[..3]);
    bytes.extend(&(height - 1).to_le_bytes()[..3]);
    bytes
}

#[test]
fn too_many_pixels_are_refused_from_the_header_alone() {
    let headers = [
        png_header((20_000, 20_000)),
        jpeg_header((9_000, 9_000)),
        webp_header((10_000, 10_000)),
    ];
    for bytes in headers {
        let error = vision::inspect(&bytes).unwrap_err();
        assert!(
            matches!(error, VisionError::ImageTooLarge { max_pixels, .. } if max_pixels == MAX_PIXELS),
            "{error}"
        );
        assert_eq!(error.code(), "CAPTURE_IMAGE_TOO_LARGE");
        // run() refuses it the same way, never decoding.
        let error = vision::run((&photo_input(), &bytes)).unwrap_err();
        assert_eq!(error.code(), "CAPTURE_IMAGE_TOO_LARGE");
    }
    // The headers are read right: a size within the limit passes inspection.
    assert_eq!(
        vision::inspect(&jpeg_header((4000, 3000))).unwrap().width,
        4000
    );
    assert_eq!(
        vision::inspect(&webp_header((4000, 3000))).unwrap().height,
        3000
    );
    assert_eq!(
        vision::inspect(&png_header((6000, 4000))).unwrap().width,
        6000
    );
}

#[test]
fn small_broken_and_foreign_files_are_refused() -> TestResult {
    let small = encode((&plain((800, 600))?, ".png"))?;
    let error = vision::inspect(&small).unwrap_err();
    assert!(
        matches!(error, VisionError::ImageTooSmall { min_long_side, .. } if min_long_side == MIN_LONG_SIDE_PX)
    );
    assert_eq!(error.code(), "CAPTURE_IMAGE_TOO_SMALL");

    let foreign: [&[u8]; 4] = [
        b"",
        b"GIF89a\x10\x00\x10\x00",
        b"\0\0\0\x18ftypheic\0\0\0\0mif1heic",
        b"%PDF-1.7\n",
    ];
    for bytes in foreign {
        assert_eq!(
            vision::inspect(bytes).unwrap_err().code(),
            "CAPTURE_UNSUPPORTED_IMAGE"
        );
    }

    // Cut short, or a PNG whose first chunk isn't its header.
    let full = encode((&plain((1200, 900))?, ".jpg"))?;
    let mut wrong_chunk = png_header((1200, 900));
    wrong_chunk[12..16].copy_from_slice(b"IDAT");
    for bytes in [&full[..4], &wrong_chunk[..]] {
        assert_eq!(
            vision::inspect(bytes).unwrap_err().code(),
            "CAPTURE_UNREADABLE_IMAGE"
        );
    }
    // A header with nothing behind it passes inspection but can't be decoded.
    let error = vision::decode(&png_header((1200, 900))).unwrap_err();
    assert_eq!(error.code(), "CAPTURE_UNREADABLE_IMAGE");
    // Damaged pixels: the frame is cut off halfway.
    let cut = &full[..full.len() / 2];
    match vision::decode(cut) {
        // libjpeg may fill the missing half with grey; the size still matches.
        Ok(image) => assert_eq!((image.cols(), image.rows()), (1200, 900)),
        Err(error) => assert_eq!(error.code(), "CAPTURE_UNREADABLE_IMAGE"),
    }
    Ok(())
}

/// A JPEG with an EXIF orientation tag added right after its start marker.
fn with_orientation((jpeg, orientation): (&[u8], u16)) -> Vec<u8> {
    let mut tiff = b"II*\0".to_vec();
    tiff.extend(8_u32.to_le_bytes());
    tiff.extend(1_u16.to_le_bytes());
    // Orientation (0x0112), a SHORT, one value, padded to 4 bytes; then no next IFD.
    tiff.extend(0x0112_u16.to_le_bytes());
    tiff.extend(3_u16.to_le_bytes());
    tiff.extend(1_u32.to_le_bytes());
    tiff.extend(orientation.to_le_bytes());
    tiff.extend([0, 0]);
    tiff.extend(0_u32.to_le_bytes());
    let mut segment = b"Exif\0\0".to_vec();
    segment.extend(tiff);
    let length = u16::try_from(segment.len() + 2).unwrap();
    let mut bytes = jpeg[..2].to_vec();
    bytes.extend([0xFF, 0xE1]);
    bytes.extend(length.to_be_bytes());
    bytes.extend(segment);
    bytes.extend(&jpeg[2..]);
    bytes
}

#[test]
fn exif_orientation_turns_the_photo_upright() -> TestResult {
    let photo = disc_photo()?;
    let jpeg = encode((&photo, ".jpg"))?;
    let plain = vision::decode(&jpeg)?;
    assert_eq!((plain.cols(), plain.rows()), (4000, 3000));
    // Orientation 6: the camera was turned, so the stored picture needs a quarter turn
    // clockwise to stand upright.
    let turned = with_orientation((&jpeg, 6));
    assert_eq!(
        vision::inspect(&turned)?.width,
        4000,
        "the header's size, as stored"
    );
    let upright = vision::decode(&turned)?;
    assert_eq!((upright.cols(), upright.rows()), (3000, 4000));
    // The table's dark top-left corner of the stored picture is now top-right.
    let corner = |image: &Mat, (x, y): (i32, i32)| -> opencv::Result<i32> {
        let pixel = image.at_2d::<core::Vec3b>(y, x)?;
        Ok(i32::from(pixel[0]) + i32::from(pixel[1]) + i32::from(pixel[2]))
    };
    assert_eq!(corner(&upright, (2999, 0))?, corner(&plain, (0, 0))?);
    // And it measures the same.
    let result = vision::run((&photo_input(), &turned))?;
    assert_close(
        "turned diameter",
        result.products[0].diameter_mm.ok_or("no diameter")?,
        52.0,
    );
    Ok(())
}

#[test]
fn the_input_reads_from_json_and_is_checked_first() -> TestResult {
    let input: CaptureInput = serde_json::from_value(json!({ "paper": "a4", "mode": "trace" }))?;
    assert_eq!(
        input,
        CaptureInput {
            paper: PaperSize::A4,
            mode: CaptureMode::Trace,
            pen_width_mm: 0.5,
            tap_point_px: None,
            height_mm: None,
            focal_length_35mm: None,
        }
    );
    let input: CaptureInput =
        serde_json::from_value(json!({ "paper": "Letter", "mode": "photo", "pen_width_mm": 1.2 }))?;
    assert_eq!((input.paper, input.pen_width_mm), (PaperSize::Letter, 1.2));
    assert!(
        serde_json::from_value::<CaptureInput>(json!({ "paper": "A3", "mode": "photo" })).is_err()
    );

    for pen_width_mm in [0.05, 3.5, f64::NAN] {
        let input = CaptureInput {
            pen_width_mm,
            ..photo_input()
        };
        let error = input.validate().unwrap_err();
        assert!(matches!(
            error,
            VisionError::InvalidInput {
                field: "pen_width_mm",
                ..
            }
        ));
        assert_eq!(error.code(), "CAPTURE_INVALID_INPUT");
        // Checked before the file is even looked at.
        let error = vision::run((&input, b"not an image")).unwrap_err();
        assert_eq!(error.code(), "CAPTURE_INVALID_INPUT");
    }
    Ok(())
}

#[test]
fn the_result_has_the_documented_shape_and_reads_back() -> TestResult {
    let bytes = encode((&disc_photo()?, ".jpg"))?;
    let result = vision::run((&photo_input(), &bytes))?;
    let value = serde_json::to_value(&result)?;

    // CLAUDE.md §5.5.
    assert_eq!(value["paper"]["size"], "A4");
    assert_eq!(
        value["paper"]["corners_px"].as_array().map(Vec::len),
        Some(4)
    );
    assert_eq!(value["paper"]["px_per_mm"], 10.0);
    let product = &value["products"][0];
    assert_eq!(product["id"], "p1");
    assert_eq!(product["shape"], "round");
    assert!(
        product["confidence"]
            .as_f64()
            .is_some_and(|c| (0.0..=1.0).contains(&c))
    );
    let diameter = product["diameter_mm"].as_f64().ok_or("no diameter")?;
    assert!((diameter - 52.0).abs() <= TOLERANCE_MM);
    assert!(product["bbox_mm"]["l"].is_number() && product["bbox_mm"]["w"].is_number());
    let contour = product["contour_mm"].as_array().ok_or("no contour")?;
    assert!(
        contour.len() >= 8
            && contour
                .iter()
                .all(|point| point.as_array().map(Vec::len) == Some(2))
    );
    // The same outline in photo pixels, for the editor (M1-C3).
    assert_eq!(
        product["contour_px"].as_array().map(Vec::len),
        Some(contour.len())
    );
    assert_eq!(product["contour_source"], "detected");
    assert!(product["height_mm"].is_null());
    assert_eq!(product["height_source"], "user");
    assert_eq!(value["level_used"], "L1");
    assert_eq!(value["mode"], "photo");
    // Warnings are codes the editor translates.
    assert_eq!(
        value["warnings"],
        json!([{ "kind": "tall_product_looks_bigger" }])
    );

    let back: VisionResult = serde_json::from_value(value)?;
    assert_eq!(back, result);
    Ok(())
}

#[test]
fn error_codes_are_stable_and_say_whose_to_fix() {
    let errors = [
        (VisionError::UnsupportedImage, "CAPTURE_UNSUPPORTED_IMAGE"),
        (VisionError::UnreadableImage, "CAPTURE_UNREADABLE_IMAGE"),
        (
            VisionError::ImageTooLarge {
                width: 1,
                height: 1,
                max_pixels: 1,
            },
            "CAPTURE_IMAGE_TOO_LARGE",
        ),
        (
            VisionError::ImageTooSmall {
                width: 1,
                height: 1,
                min_long_side: 1,
            },
            "CAPTURE_IMAGE_TOO_SMALL",
        ),
        (VisionError::PaperNotFound, "CAPTURE_PAPER_NOT_FOUND"),
        (VisionError::ProductNotFound, "CAPTURE_PRODUCT_NOT_FOUND"),
        (VisionError::TraceNotFound, "CAPTURE_TRACE_NOT_FOUND"),
        (VisionError::NothingAtTap, "CAPTURE_NOTHING_AT_TAP"),
        (
            VisionError::OutlineCrossesItself,
            "CAPTURE_OUTLINE_CROSSES_ITSELF",
        ),
    ];
    for (error, code) in errors {
        assert_eq!(error.code(), code);
        assert!(error.is_users_to_fix(), "{code}");
    }
    let internal = VisionError::OpenCv(opencv::Error::new(0, "test"));
    assert_eq!(internal.code(), "CAPTURE_PROCESSING_FAILED");
    assert!(!internal.is_users_to_fix());
}
