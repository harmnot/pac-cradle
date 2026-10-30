//! The one EXIF field a cleaned photo keeps: its orientation (tag 0x0112), which says how to
//! turn the stored picture upright. It's read from the first image directory of the TIFF
//! block every format wraps EXIF in, and written back as a TIFF block holding nothing else.

/// The marker before the TIFF block in a JPEG's APP1 segment (and in some WebP EXIF chunks).
pub(crate) const EXIF_ID: &[u8] = b"Exif\0\0";

const ORIENTATION_TAG: u16 = 0x0112;
/// TIFF field type SHORT: 16-bit unsigned.
const SHORT: u16 = 3;
/// Bytes per directory entry: tag, type, count, value or offset.
const ENTRY_LEN: usize = 12;

/// The orientation (1 to 8) in a TIFF block, or `None` when it has none or can't be read.
pub(crate) fn orientation(tiff: &[u8]) -> Option<u16> {
    let tiff = tiff.strip_prefix(EXIF_ID).unwrap_or(tiff);
    let order = match tiff.get(..4)? {
        b"II*\0" => Order::Little,
        b"MM\0*" => Order::Big,
        _ => return None,
    };
    let directory = usize::try_from(order.u32(tiff, 4)?).ok()?;
    let count = usize::from(order.u16(tiff, directory)?);
    (0..count)
        .map(|entry| directory + 2 + entry * ENTRY_LEN)
        .find(|&at| order.u16(tiff, at) == Some(ORIENTATION_TAG))
        .filter(|&at| order.u16(tiff, at + 2) == Some(SHORT))
        .and_then(|at| order.u16(tiff, at + 8))
        .filter(|value| (1..=8).contains(value))
}

/// A TIFF block whose only field is `orientation`: 26 bytes, big-endian.
pub(crate) fn orientation_tiff(orientation: u16) -> Vec<u8> {
    let mut tiff = Vec::with_capacity(26);
    tiff.extend(b"MM\0*");
    // The image directory starts right after the 8-byte header.
    tiff.extend(8_u32.to_be_bytes());
    tiff.extend(1_u16.to_be_bytes());
    tiff.extend(ORIENTATION_TAG.to_be_bytes());
    tiff.extend(SHORT.to_be_bytes());
    tiff.extend(1_u32.to_be_bytes());
    // One SHORT, left-aligned in the 4-byte value field.
    tiff.extend(orientation.to_be_bytes());
    tiff.extend([0, 0]);
    // No next directory, so no thumbnail.
    tiff.extend(0_u32.to_be_bytes());
    tiff
}

/// The orientation worth keeping: any but 1, which means "already upright".
pub(crate) fn turned(orientation: Option<u16>) -> Option<u16> {
    orientation.filter(|&value| value != 1)
}

#[derive(Clone, Copy)]
enum Order {
    Little,
    Big,
}

impl Order {
    fn u16(self, bytes: &[u8], at: usize) -> Option<u16> {
        let pair: [u8; 2] = bytes.get(at..at.checked_add(2)?)?.try_into().ok()?;
        Some(match self {
            Self::Little => u16::from_le_bytes(pair),
            Self::Big => u16::from_be_bytes(pair),
        })
    }

    fn u32(self, bytes: &[u8], at: usize) -> Option<u32> {
        let quad: [u8; 4] = bytes.get(at..at.checked_add(4)?)?.try_into().ok()?;
        Some(match self {
            Self::Little => u32::from_le_bytes(quad),
            Self::Big => u32::from_be_bytes(quad),
        })
    }
}
