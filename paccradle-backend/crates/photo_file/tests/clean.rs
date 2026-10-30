//! M1-D4: uploads are checked from their header, then kept without their metadata. Files are
//! built by hand here, each carrying GPS, a camera name, XMP, comments, thumbnails or data
//! after the image, all marked `SECRET`; none of it may survive, while the picture's own bytes,
//! its colour profile and its orientation must. (`vision`'s tests decode cleaned photos.)
// The helpers below are test code too; clippy.toml only exempts `#[test]` functions.
#![allow(clippy::unwrap_used)]

use photo_file::{ImageFormat, ImageInfo, MAX_PIXELS, MIN_LONG_SIDE_PX, PhotoError};

const SECRET: &[u8] = b"SECRET";

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

fn assert_no_secret(bytes: &[u8]) {
    assert!(!contains(bytes, SECRET), "metadata survived cleaning");
}

/// A TIFF entry: tag, type, count, value (or offset).
fn entry(tiff: &mut Vec<u8>, (tag, kind, count, value): (u16, u16, u32, [u8; 4])) {
    tiff.extend(tag.to_le_bytes());
    tiff.extend(kind.to_le_bytes());
    tiff.extend(count.to_le_bytes());
    tiff.extend(value);
}

/// What a phone writes, little-endian: the camera's name, the orientation, a GPS directory
/// with a latitude, and a thumbnail in a second directory.
fn camera_tiff(orientation: u16) -> Vec<u8> {
    let mut tiff = b"II*\0".to_vec();
    tiff.extend(8_u32.to_le_bytes());
    tiff.extend(3_u16.to_le_bytes());
    entry(&mut tiff, (0x010F, 2, 13, 50_u32.to_le_bytes()));
    let [low, high] = orientation.to_le_bytes();
    entry(&mut tiff, (0x0112, 3, 1, [low, high, 0, 0]));
    entry(&mut tiff, (0x8825, 4, 1, 64_u32.to_le_bytes()));
    tiff.extend(118_u32.to_le_bytes());
    assert_eq!(tiff.len(), 50);
    tiff.extend(b"SECRET-PHONE\0\0");
    // The GPS directory.
    tiff.extend(2_u16.to_le_bytes());
    entry(&mut tiff, (0x0001, 2, 2, *b"N\0\0\0"));
    entry(&mut tiff, (0x0002, 5, 3, 94_u32.to_le_bytes()));
    tiff.extend(0_u32.to_le_bytes());
    assert_eq!(tiff.len(), 94);
    tiff.extend(b"SECRET-LATITUDE-SECRET!!");
    // The thumbnail's directory.
    tiff.extend(2_u16.to_le_bytes());
    entry(&mut tiff, (0x0201, 4, 1, 148_u32.to_le_bytes()));
    entry(&mut tiff, (0x0202, 4, 1, 16_u32.to_le_bytes()));
    tiff.extend(0_u32.to_le_bytes());
    assert_eq!(tiff.len(), 148);
    tiff.extend(b"SECRET-THUMBNAIL");
    tiff
}

/// The TIFF block a cleaned photo carries: big-endian, the orientation alone.
fn orientation_only(orientation: u16) -> Vec<u8> {
    let mut tiff = b"MM\0*\0\0\0\x08\0\x01\x01\x12\0\x03\0\0\0\x01".to_vec();
    tiff.extend(orientation.to_be_bytes());
    tiff.extend([0; 6]);
    tiff
}

// --- JPEG ---

fn segment(marker: u8, payload: &[u8]) -> Vec<u8> {
    let mut segment = vec![0xFF, marker];
    segment.extend(u16::try_from(payload.len() + 2).unwrap().to_be_bytes());
    segment.extend(payload);
    segment
}

fn exif_segment(tiff: &[u8]) -> Vec<u8> {
    segment(0xE1, &[b"Exif\0\0".as_slice(), tiff].concat())
}

/// The segments that draw a 4000 × 3000 progressive picture: tables, frame, two scans with
/// stuffed bytes and a restart marker, and the end. Not a decodable image: `vision` decodes
/// real ones.
fn picture() -> Vec<u8> {
    let mut frame = vec![8];
    frame.extend(3000_u16.to_be_bytes());
    frame.extend(4000_u16.to_be_bytes());
    frame.push(3);
    for id in 1..=3 {
        frame.extend([id, 0x11, 0]);
    }
    let scan_header = segment(0xDA, &[3, 1, 0, 2, 0x11, 3, 0x11, 0, 63, 0]);
    [
        segment(0xDB, &[0; 65]),
        segment(0xC2, &frame),
        segment(0xC4, &[0x00, 1, 2, 3]),
        segment(0xDD, &[0, 64]),
        scan_header.clone(),
        b"\x12\x34\xFF\x00\x56\xFF\xD0\x78".to_vec(),
        segment(0xC4, &[0x10, 4, 5, 6]),
        scan_header,
        b"\x9A\xFF\x00\xBC".to_vec(),
        vec![0xFF, 0xD9],
    ]
    .concat()
}

/// The colour segments a cleaned JPEG keeps: an ICC profile and Adobe's colour transform.
fn colour_segments() -> Vec<u8> {
    [
        segment(0xE2, b"ICC_PROFILE\0\x01\x01fake profile"),
        segment(0xEE, b"Adobe\0\x64\0\0\0\0\x01"),
    ]
    .concat()
}

/// A phone's JPEG: JFIF with a thumbnail, EXIF with GPS, XMP, the colour segments, IPTC, a
/// comment, the picture, then a second image (a gain map) after the end.
fn phone_jpeg(orientation: u16) -> Vec<u8> {
    [
        vec![0xFF, 0xD8],
        segment(0xE0, b"JFIF\0\x01\x02\0\0\x01\0\x01\x01\x01SECRET-JFIF"),
        exif_segment(&camera_tiff(orientation)),
        segment(
            0xE1,
            b"http://ns.adobe.com/xap/1.0/\0<x:xmpmeta exif:GPSLatitude='SECRET'/>",
        ),
        colour_segments(),
        segment(0xED, b"Photoshop 3.0\08BIM SECRET-IPTC"),
        segment(0xFE, b"SECRET-COMMENT"),
        picture(),
        b"\xFF\xD8\xFF\xE1\0\x10SECRET-GAINMAP\xFF\xD9".to_vec(),
    ]
    .concat()
}

#[test]
fn a_jpeg_keeps_its_picture_colours_and_orientation_and_nothing_else() {
    let cleaned = photo_file::clean(&phone_jpeg(6)).unwrap();
    assert_eq!(
        cleaned.info,
        ImageInfo {
            format: ImageFormat::Jpeg,
            width: 4000,
            height: 3000
        }
    );
    let expected = [
        vec![0xFF, 0xD8],
        exif_segment(&orientation_only(6)),
        colour_segments(),
        picture(),
    ]
    .concat();
    assert_eq!(cleaned.bytes, expected);
    assert_no_secret(&cleaned.bytes);
    assert_eq!(photo_file::inspect(&cleaned.bytes), Ok(cleaned.info));
}

#[test]
fn an_upright_or_unturned_jpeg_gets_no_exif_at_all() {
    let expected = [vec![0xFF, 0xD8], colour_segments(), picture()].concat();
    // Orientation 1 means already upright.
    assert_eq!(photo_file::clean(&phone_jpeg(1)).unwrap().bytes, expected);
    // No EXIF in the first place.
    let plain = [vec![0xFF, 0xD8], colour_segments(), picture()].concat();
    assert_eq!(photo_file::clean(&plain).unwrap().bytes, expected);
}

#[test]
fn a_big_endian_orientation_is_read_too() {
    let mut big_endian = orientation_only(8);
    // Another field before the orientation, so it isn't simply the first entry.
    big_endian.splice(
        8..10,
        [0, 2, 0x01, 0x0F, 0, 2, 0, 0, 0, 4, b'S', b'E', b'C', 0],
    );
    let jpeg = [vec![0xFF, 0xD8], exif_segment(&big_endian), picture()].concat();
    let cleaned = photo_file::clean(&jpeg).unwrap().bytes;
    assert_eq!(
        cleaned,
        [
            vec![0xFF, 0xD8],
            exif_segment(&orientation_only(8)),
            picture()
        ]
        .concat()
    );
}

#[test]
fn fill_bytes_before_a_marker_are_dropped() {
    let mut jpeg = [vec![0xFF, 0xD8], picture()].concat();
    // Fill bytes before the end marker, and before a segment.
    let end = jpeg.len() - 2;
    jpeg.splice(end..end, [0xFF, 0xFF]);
    jpeg.splice(2..2, [0xFF]);
    let cleaned = photo_file::clean(&jpeg).unwrap().bytes;
    assert_eq!(cleaned, [vec![0xFF, 0xD8], picture()].concat());
}

#[test]
fn a_jpeg_without_its_end_marker_is_unreadable() {
    let mut jpeg = [vec![0xFF, 0xD8], picture()].concat();
    jpeg.truncate(jpeg.len() - 2);
    assert_eq!(photo_file::clean(&jpeg), Err(PhotoError::UnreadableImage));
}

// --- PNG ---

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFF_u32;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

fn chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut chunk = u32::try_from(data.len()).unwrap().to_be_bytes().to_vec();
    chunk.extend(kind);
    chunk.extend(data);
    chunk.extend(crc32(&[kind.as_slice(), data].concat()).to_be_bytes());
    chunk
}

fn ihdr() -> Vec<u8> {
    let mut data = 4000_u32.to_be_bytes().to_vec();
    data.extend(3000_u32.to_be_bytes());
    data.extend([8, 2, 0, 0, 0]);
    chunk(b"IHDR", &data)
}

/// The chunks a cleaned PNG keeps after IHDR (and its new eXIf).
fn png_picture() -> Vec<u8> {
    [
        chunk(b"sRGB", &[0]),
        chunk(b"iCCP", b"profile\0\0compressed"),
        chunk(b"pHYs", &[0, 0, 0x0B, 0x13, 0, 0, 0x0B, 0x13, 1]),
        chunk(b"IDAT", b"first half of the pixels"),
        chunk(b"IDAT", b"second half"),
        chunk(b"IEND", &[]),
    ]
    .concat()
}

#[test]
fn a_png_keeps_its_picture_chunks_and_orientation_and_nothing_else() {
    let png = [
        b"\x89PNG\r\n\x1a\n".to_vec(),
        ihdr(),
        chunk(b"eXIf", &camera_tiff(3)),
        chunk(b"tEXt", b"Comment\0SECRET-COMMENT"),
        chunk(
            b"iTXt",
            b"XML:com.adobe.xmp\0\0\0\0\0<x:xmpmeta GPS='SECRET'/>",
        ),
        chunk(b"tIME", &[7, 234, 10, 30, 12, 0, 0]),
        chunk(b"prVt", b"SECRET-PRIVATE-CHUNK"),
        png_picture(),
        b"SECRET-TRAILER".to_vec(),
    ]
    .concat();
    let cleaned = photo_file::clean(&png).unwrap();
    assert_eq!(cleaned.info.format, ImageFormat::Png);
    let new_exif = chunk(b"eXIf", &orientation_only(3));
    let expected = [
        b"\x89PNG\r\n\x1a\n".to_vec(),
        ihdr(),
        new_exif,
        png_picture(),
    ]
    .concat();
    assert_eq!(cleaned.bytes, expected, "the new eXIf's CRC is checked too");
    assert_no_secret(&cleaned.bytes);
    assert_eq!(photo_file::clean(&cleaned.bytes).unwrap(), cleaned);
}

// --- WebP ---

fn riff(chunks: &[Vec<u8>]) -> Vec<u8> {
    let body = chunks.concat();
    let mut file = b"RIFF".to_vec();
    file.extend(u32::try_from(4 + body.len()).unwrap().to_le_bytes());
    file.extend(b"WEBP");
    file.extend(body);
    file
}

fn webp_chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut chunk = kind.to_vec();
    chunk.extend(u32::try_from(data.len()).unwrap().to_le_bytes());
    chunk.extend(data);
    if data.len() % 2 == 1 {
        chunk.push(0);
    }
    chunk
}

fn vp8x(flags: u8) -> Vec<u8> {
    let mut data = vec![flags, 0, 0, 0];
    data.extend(&3999_u32.to_le_bytes()[..3]);
    data.extend(&2999_u32.to_le_bytes()[..3]);
    webp_chunk(b"VP8X", &data)
}

/// A lossy key frame of 4000 × 3000.
fn vp8() -> Vec<u8> {
    let mut data = vec![0x50, 0x2A, 0x00, 0x9D, 0x01, 0x2A];
    data.extend(4000_u16.to_le_bytes());
    data.extend(3000_u16.to_le_bytes());
    data.extend(b"pixels");
    webp_chunk(b"VP8 ", &data)
}

const ICC: u8 = 0x20;
const EXIF: u8 = 0x08;
const XMP: u8 = 0x04;

#[test]
fn an_extended_webp_keeps_its_picture_and_orientation_and_nothing_else() {
    let profile = webp_chunk(b"ICCP", b"odd profile");
    let mut webp = riff(&[
        vp8x(ICC | EXIF | XMP),
        profile.clone(),
        vp8(),
        webp_chunk(b"EXIF", &camera_tiff(8)),
        webp_chunk(b"XMP ", b"<x:xmpmeta GPS='SECRET'/>"),
        webp_chunk(b"ZZZZ", b"SECRET-UNKNOWN"),
    ]);
    webp.extend(b"SECRET-AFTER-THE-RIFF");
    let cleaned = photo_file::clean(&webp).unwrap();
    assert_eq!(
        cleaned.info,
        ImageInfo {
            format: ImageFormat::Webp,
            width: 4000,
            height: 3000
        }
    );
    let expected = riff(&[
        vp8x(ICC | EXIF),
        profile.clone(),
        vp8(),
        webp_chunk(b"EXIF", &orientation_only(8)),
    ]);
    assert_eq!(cleaned.bytes, expected);
    assert_no_secret(&cleaned.bytes);

    // Upright: no EXIF chunk and no EXIF flag.
    let upright = riff(&[
        vp8x(ICC | EXIF),
        profile.clone(),
        vp8(),
        webp_chunk(b"EXIF", &camera_tiff(1)),
    ]);
    assert_eq!(
        photo_file::clean(&upright).unwrap().bytes,
        riff(&[vp8x(ICC), profile, vp8()])
    );
}

#[test]
fn a_simple_webp_keeps_only_its_image_chunk() {
    let webp = riff(&[vp8(), webp_chunk(b"EXIF", &camera_tiff(6))]);
    assert_eq!(photo_file::clean(&webp).unwrap().bytes, riff(&[vp8()]));
}

#[test]
fn a_webp_whose_chunks_run_past_its_end_is_unreadable() {
    let mut webp = riff(&[vp8x(0), vp8()]);
    // The RIFF size says 100 bytes more than there are.
    let size = u32::from_le_bytes(webp[4..8].try_into().unwrap()) + 100;
    webp[4..8].copy_from_slice(&size.to_le_bytes());
    assert_eq!(photo_file::clean(&webp), Err(PhotoError::UnreadableImage));
    // A VP8X too short to hold its flags.
    let truncated = riff(&[webp_chunk(b"VP8X", &[]), vp8()]);
    assert!(photo_file::clean(&truncated).is_err());
}

// --- Every format ---

fn samples() -> Vec<Vec<u8>> {
    vec![
        phone_jpeg(6),
        [
            b"\x89PNG\r\n\x1a\n".to_vec(),
            ihdr(),
            chunk(b"eXIf", &camera_tiff(3)),
            png_picture(),
        ]
        .concat(),
        riff(&[
            vp8x(ICC | EXIF | XMP),
            webp_chunk(b"ICCP", b"odd profile"),
            vp8(),
            webp_chunk(b"EXIF", &camera_tiff(8)),
            webp_chunk(b"XMP ", b"<x:xmpmeta/>"),
        ]),
    ]
}

#[test]
fn cleaning_twice_changes_nothing_more() {
    for sample in samples() {
        let once = photo_file::clean(&sample).unwrap();
        assert_eq!(photo_file::clean(&once.bytes).unwrap(), once);
    }
}

#[test]
fn files_cut_short_or_damaged_are_refused_without_panicking() {
    for sample in samples() {
        for length in 0..sample.len() {
            // Any answer will do, as long as it is one.
            let _ = photo_file::clean(&sample[..length]);
        }
        for at in 0..sample.len() {
            for value in [0x00, 0xFF, 0x7F] {
                let mut damaged = sample.clone();
                damaged[at] = value;
                let _ = photo_file::clean(&damaged);
            }
        }
    }
}

// --- Headers ---

#[test]
fn inspect_reads_the_format_and_size_and_refuses_the_rest() {
    let [jpeg, png, webp]: [Vec<u8>; 3] = samples().try_into().unwrap();
    for (bytes, format) in [
        (jpeg, ImageFormat::Jpeg),
        (png, ImageFormat::Png),
        (webp, ImageFormat::Webp),
    ] {
        assert_eq!(
            photo_file::inspect(&bytes),
            Ok(ImageInfo {
                format,
                width: 4000,
                height: 3000
            })
        );
    }
    // A lossless WebP: a signature byte, then the width and height less one, 14 bits each.
    let bits: u32 = (1200 - 1) | ((900 - 1) << 14);
    let mut data = vec![0x2F];
    data.extend(bits.to_le_bytes());
    let lossless = riff(&[webp_chunk(b"VP8L", &data)]);
    assert_eq!(photo_file::inspect(&lossless).unwrap().width, 1200);

    // 8000 × 5001 is just over the pixel cap; 999 px on the long side just under the minimum.
    let huge = [
        b"\x89PNG\r\n\x1a\n".to_vec(),
        chunk(
            b"IHDR",
            &[0, 0, 0x1F, 0x40, 0, 0, 0x13, 0x89, 8, 2, 0, 0, 0],
        ),
    ]
    .concat();
    assert_eq!(
        photo_file::inspect(&huge),
        Err(PhotoError::ImageTooLarge {
            width: 8000,
            height: 5001,
            max_pixels: MAX_PIXELS
        })
    );
    let tiny = [
        b"\x89PNG\r\n\x1a\n".to_vec(),
        chunk(
            b"IHDR",
            &[0, 0, 0x03, 0xE7, 0, 0, 0x02, 0xEE, 8, 2, 0, 0, 0],
        ),
    ]
    .concat();
    assert_eq!(
        photo_file::inspect(&tiny),
        Err(PhotoError::ImageTooSmall {
            width: 999,
            height: 750,
            min_long_side: MIN_LONG_SIDE_PX
        })
    );
    // Too large is refused before anything else is read.
    assert_eq!(
        photo_file::clean(&huge).unwrap_err().code(),
        "CAPTURE_IMAGE_TOO_LARGE"
    );

    for foreign in [
        b"GIF89a".as_slice(),
        b"\0\0\0\x18ftypheic",
        b"%PDF-1.7",
        b"",
    ] {
        assert_eq!(
            photo_file::inspect(foreign),
            Err(PhotoError::UnsupportedImage)
        );
    }
    assert_eq!(
        photo_file::inspect(&[0xFF, 0xD8, 0xFF, 0xD9])
            .unwrap_err()
            .code(),
        "CAPTURE_UNREADABLE_IMAGE"
    );
}

#[test]
fn formats_name_their_file_extension_and_media_type() {
    assert_eq!(ImageFormat::Jpeg.extension(), "jpg");
    assert_eq!(ImageFormat::Png.media_type(), "image/png");
    assert_eq!(ImageFormat::Webp.media_type(), "image/webp");
}
