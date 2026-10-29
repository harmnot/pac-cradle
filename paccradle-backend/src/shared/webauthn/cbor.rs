//! A CBOR reader (RFC 8949) for what passkeys send: the attestation object, the COSE public key
//! and authenticator extensions. Authenticators write CTAP2 canonical CBOR, so only
//! definite-length items are read; tags, floats and indefinite lengths are refused. Item counts
//! are checked against the bytes left and nesting is capped, so no input can make it allocate
//! or recurse without bound.

/// How deep arrays and maps may nest. A COSE key is 1 level, an attestation object 2.
const MAX_DEPTH: usize = 8;

/// A CBOR data item, borrowing byte and text strings from the input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value<'a> {
    /// An unsigned integer.
    Unsigned(u64),
    /// A negative integer: the value is `-1 - n`.
    Negative(u64),
    /// A byte string.
    Bytes(&'a [u8]),
    /// A UTF-8 text string.
    Text(&'a str),
    /// An array.
    Array(Vec<Value<'a>>),
    /// A map, its pairs in the order written.
    Map(Vec<(Value<'a>, Value<'a>)>),
    /// `true` or `false`.
    Bool(bool),
    /// `null`.
    Null,
}

/// Why the bytes aren't CBOR this reader accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("bad CBOR: {0}")]
pub struct CborError(pub &'static str);

impl<'a> Value<'a> {
    /// The value as an `i64`, if it is an integer that fits.
    pub fn as_int(&self) -> Option<i64> {
        match *self {
            Self::Unsigned(n) => i64::try_from(n).ok(),
            Self::Negative(n) => i64::try_from(n).ok().map(|n| -1 - n),
            _ => None,
        }
    }

    /// The value as a byte string.
    pub fn as_bytes(&self) -> Option<&'a [u8]> {
        match *self {
            Self::Bytes(bytes) => Some(bytes),
            _ => None,
        }
    }

    /// The value as a text string.
    pub fn as_text(&self) -> Option<&'a str> {
        match *self {
            Self::Text(text) => Some(text),
            _ => None,
        }
    }

    /// In a map, the value under the integer key `key`.
    pub fn get_int(&self, key: i64) -> Option<&Value<'a>> {
        self.find(|k| k.as_int() == Some(key))
    }

    /// In a map, the value under the text key `key`.
    pub fn get_text(&self, key: &str) -> Option<&Value<'a>> {
        self.find(|k| k.as_text() == Some(key))
    }

    fn find(&self, matches: impl Fn(&Value<'a>) -> bool) -> Option<&Value<'a>> {
        match self {
            Self::Map(pairs) => pairs.iter().find(|(k, _)| matches(k)).map(|(_, v)| v),
            _ => None,
        }
    }
}

/// The single item that makes up all of `input`.
pub fn decode(input: &[u8]) -> Result<Value<'_>, CborError> {
    let (value, used) = decode_prefix(input)?;
    if used != input.len() {
        return Err(CborError("bytes after the item"));
    }
    Ok(value)
}

/// The item at the start of `input`, and how many bytes it takes.
pub fn decode_prefix(input: &[u8]) -> Result<(Value<'_>, usize), CborError> {
    let mut reader = Reader { input, pos: 0 };
    let value = reader.item(0)?;
    Ok((value, reader.pos))
}

struct Reader<'a> {
    input: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn remaining(&self) -> usize {
        self.input.len() - self.pos
    }

    fn take(&mut self, len: u64) -> Result<&'a [u8], CborError> {
        let len = usize::try_from(len).map_err(|_| CborError("length too large"))?;
        if len > self.remaining() {
            return Err(CborError("ends early"));
        }
        let bytes = &self.input[self.pos..self.pos + len];
        self.pos += len;
        Ok(bytes)
    }

    fn argument(&mut self, info: u8) -> Result<u64, CborError> {
        let size = match info {
            0..=23 => return Ok(u64::from(info)),
            24 => 1,
            25 => 2,
            26 => 4,
            27 => 8,
            _ => return Err(CborError("indefinite length or reserved value")),
        };
        let bytes = self.take(size)?;
        Ok(bytes.iter().fold(0, |n, &b| n << 8 | u64::from(b)))
    }

    /// An item count, which can't be more than the bytes left (each item takes at least one).
    fn count(&mut self, info: u8, per_item: u64) -> Result<usize, CborError> {
        let count = self.argument(info)?;
        let needed = count
            .checked_mul(per_item)
            .ok_or(CborError("too many items"))?;
        if needed > self.remaining() as u64 {
            return Err(CborError("ends early"));
        }
        usize::try_from(count).map_err(|_| CborError("too many items"))
    }

    fn item(&mut self, depth: usize) -> Result<Value<'a>, CborError> {
        if depth > MAX_DEPTH {
            return Err(CborError("nested too deeply"));
        }
        let initial = self.take(1)?[0];
        let (major, info) = (initial >> 5, initial & 0x1f);
        match major {
            0 => Ok(Value::Unsigned(self.argument(info)?)),
            1 => Ok(Value::Negative(self.argument(info)?)),
            2 => {
                let len = self.argument(info)?;
                Ok(Value::Bytes(self.take(len)?))
            }
            3 => {
                let len = self.argument(info)?;
                let bytes = self.take(len)?;
                std::str::from_utf8(bytes)
                    .map(Value::Text)
                    .map_err(|_| CborError("text isn't UTF-8"))
            }
            4 => {
                let count = self.count(info, 1)?;
                (0..count)
                    .map(|_| self.item(depth + 1))
                    .collect::<Result<_, _>>()
                    .map(Value::Array)
            }
            5 => {
                let count = self.count(info, 2)?;
                (0..count)
                    .map(|_| Ok((self.item(depth + 1)?, self.item(depth + 1)?)))
                    .collect::<Result<_, _>>()
                    .map(Value::Map)
            }
            6 => Err(CborError("tags aren't accepted")),
            _ => match info {
                20 => Ok(Value::Bool(false)),
                21 => Ok(Value::Bool(true)),
                22 => Ok(Value::Null),
                _ => Err(CborError("floats and simple values aren't accepted")),
            },
        }
    }
}
