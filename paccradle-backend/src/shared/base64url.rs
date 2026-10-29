//! Base64url without padding (RFC 4648 §5), the encoding of tokens, access-token parts and
//! every binary field of a passkey ceremony. A few lines, so no crate (vanilla-Rust rule).

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/// `bytes` as base64url, without `=` padding.
pub fn encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let n = u32::from(b[0]) << 16 | u32::from(b[1]) << 8 | u32::from(b[2]);
        // A chunk of k bytes makes k + 1 characters.
        for i in 0..=chunk.len() {
            let index = (n >> (18 - 6 * i)) & 0x3f;
            out.push(char::from(ALPHABET[index as usize]));
        }
    }
    out
}

/// The bytes in base64url `text`, or `None` if it isn't canonical unpadded base64url: padding,
/// other characters, a length no encoding makes, or unused bits that aren't zero are refused,
/// so each byte string has exactly one accepted text.
pub fn decode(text: &str) -> Option<Vec<u8>> {
    let text = text.as_bytes();
    if text.len() % 4 == 1 {
        return None;
    }
    let mut out = Vec::with_capacity(text.len() / 4 * 3 + 2);
    for chunk in text.chunks(4) {
        let mut n = 0u32;
        for (i, &c) in chunk.iter().enumerate() {
            n |= u32::from(value(c)?) << (18 - 6 * i);
        }
        let bytes = n.to_be_bytes();
        let kept = chunk.len() - 1;
        // Bits past the last whole byte must be zero.
        if bytes[1 + kept..].iter().any(|&b| b != 0) {
            return None;
        }
        out.extend_from_slice(&bytes[1..=kept]);
    }
    Some(out)
}

fn value(c: u8) -> Option<u8> {
    match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'a'..=b'z' => Some(c - b'a' + 26),
        b'0'..=b'9' => Some(c - b'0' + 52),
        b'-' => Some(62),
        b'_' => Some(63),
        _ => None,
    }
}
