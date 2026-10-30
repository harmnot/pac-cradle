//! Bounds-checked reads: anything past the end of the file is `UnreadableImage`, never a
//! panic.

use crate::PhotoError;

pub(crate) fn byte((bytes, at): (&[u8], usize)) -> Result<u8, PhotoError> {
    bytes.get(at).copied().ok_or(PhotoError::UnreadableImage)
}

/// `bytes[start..end]`, when the file reaches that far.
pub(crate) fn slice(bytes: &[u8], (start, end): (usize, usize)) -> Result<&[u8], PhotoError> {
    bytes.get(start..end).ok_or(PhotoError::UnreadableImage)
}

fn take<const N: usize>((bytes, at): (&[u8], usize)) -> Result<[u8; N], PhotoError> {
    let end = at.checked_add(N).ok_or(PhotoError::UnreadableImage)?;
    bytes
        .get(at..end)
        .and_then(|slice| slice.try_into().ok())
        .ok_or(PhotoError::UnreadableImage)
}

pub(crate) fn tag(at: (&[u8], usize)) -> Result<[u8; 4], PhotoError> {
    take::<4>(at)
}

pub(crate) fn be_u16(at: (&[u8], usize)) -> Result<u16, PhotoError> {
    take::<2>(at).map(u16::from_be_bytes)
}

pub(crate) fn be_u32(at: (&[u8], usize)) -> Result<u32, PhotoError> {
    take::<4>(at).map(u32::from_be_bytes)
}

pub(crate) fn le_u16(at: (&[u8], usize)) -> Result<u16, PhotoError> {
    take::<2>(at).map(u16::from_le_bytes)
}

pub(crate) fn le_u32(at: (&[u8], usize)) -> Result<u32, PhotoError> {
    take::<4>(at).map(u32::from_le_bytes)
}

pub(crate) fn le_u24(at: (&[u8], usize)) -> Result<u32, PhotoError> {
    take::<3>(at).map(|[low, middle, high]| u32::from_le_bytes([low, middle, high, 0]))
}

/// A length read from the file, as an index.
pub(crate) fn index(length: u32) -> Result<usize, PhotoError> {
    usize::try_from(length).map_err(|_| PhotoError::UnreadableImage)
}
