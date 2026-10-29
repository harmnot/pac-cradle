//! Authenticator data (WebAuthn §6.1): what the authenticator itself says, and what a passkey
//! signs. Layout: RP ID hash (32) ‖ flags (1) ‖ sign count (4, big-endian) ‖ the new credential
//! when registering (AAGUID 16 ‖ id length 2 ‖ id ‖ COSE key) ‖ extensions (CBOR map).

use super::WebauthnError;
use super::cbor;

/// User present: someone touched or confirmed.
pub const USER_PRESENT: u8 = 0x01;
/// User verified: biometrics, PIN or device password.
pub const USER_VERIFIED: u8 = 0x04;
/// Backup eligible: the passkey can be synced to other devices. Fixed for a credential.
pub const BACKUP_ELIGIBLE: u8 = 0x08;
/// Backed up: the passkey is synced now.
pub const BACKED_UP: u8 = 0x10;
/// Attested credential data follows (registration).
pub const ATTESTED: u8 = 0x40;
/// Extension data follows.
pub const EXTENSIONS: u8 = 0x80;

/// Bytes before the optional parts.
const FIXED_LEN: usize = 37;

/// Parsed authenticator data, borrowing from the bytes it was read from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticatorData<'a> {
    /// SHA-256 of the RP ID the authenticator used.
    pub rp_id_hash: &'a [u8],
    /// The flag byte.
    pub flags: u8,
    /// The signature counter (0 when the authenticator doesn't keep one).
    pub sign_count: u32,
    /// The new credential, on registration.
    pub credential: Option<AttestedCredential<'a>>,
}

/// The credential an authenticator just made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttestedCredential<'a> {
    /// The authenticator model's id (all zeros when it doesn't say).
    pub aaguid: [u8; 16],
    /// The credential id.
    pub credential_id: &'a [u8],
    /// The public key as COSE bytes.
    pub public_key: &'a [u8],
}

impl AuthenticatorData<'_> {
    /// Whether every bit in `flag` is set.
    pub fn has(&self, flag: u8) -> bool {
        self.flags & flag == flag
    }
}

/// Reads authenticator data. Every byte must belong to a part the flags announce.
pub fn parse(bytes: &[u8]) -> Result<AuthenticatorData<'_>, WebauthnError> {
    let short = WebauthnError("authenticator data is too short");
    if bytes.len() < FIXED_LEN {
        return Err(short);
    }
    let flags = bytes[32];
    let sign_count = u32::from_be_bytes([bytes[33], bytes[34], bytes[35], bytes[36]]);
    let mut rest = &bytes[FIXED_LEN..];

    let credential = if flags & ATTESTED != 0 {
        if rest.len() < 18 {
            return Err(short);
        }
        let mut aaguid = [0u8; 16];
        aaguid.copy_from_slice(&rest[..16]);
        let id_len = usize::from(u16::from_be_bytes([rest[16], rest[17]]));
        rest = &rest[18..];
        if id_len > rest.len() {
            return Err(short);
        }
        let (credential_id, after_id) = rest.split_at(id_len);
        let (_, key_len) = cbor::decode_prefix(after_id)
            .map_err(|_| WebauthnError("credential public key isn't CBOR"))?;
        let (public_key, after_key) = after_id.split_at(key_len);
        rest = after_key;
        Some(AttestedCredential {
            aaguid,
            credential_id,
            public_key,
        })
    } else {
        None
    };

    if flags & EXTENSIONS != 0 {
        let extensions = cbor::decode(rest)
            .map_err(|_| WebauthnError("authenticator extensions aren't one CBOR map"))?;
        if !matches!(extensions, cbor::Value::Map(_)) {
            return Err(WebauthnError("authenticator extensions aren't a map"));
        }
        rest = &[];
    }
    if !rest.is_empty() {
        return Err(WebauthnError("authenticator data has extra bytes"));
    }
    Ok(AuthenticatorData {
        rp_id_hash: &bytes[..32],
        flags,
        sign_count,
        credential,
    })
}
