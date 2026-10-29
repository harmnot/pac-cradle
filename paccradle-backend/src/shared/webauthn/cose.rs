//! Passkey public keys (COSE_Key, RFC 9052 §7) and their signatures. Three algorithms cover the
//! authenticators in use: ES256 (most), EdDSA with Ed25519, and RS256 (Windows Hello).

use ring::signature::{self, RsaPublicKeyComponents, UnparsedPublicKey};

use super::WebauthnError;
use super::cbor::{self, Value};

/// COSE algorithm ES256: ECDSA on P-256 with SHA-256.
pub const ES256: i32 = -7;
/// COSE algorithm EdDSA, here always Ed25519.
pub const EDDSA: i32 = -8;
/// COSE algorithm RS256: RSASSA-PKCS1-v1_5 with SHA-256.
pub const RS256: i32 = -257;

/// The algorithms accepted, in the order offered to authenticators.
pub const ALGORITHMS: [i32; 3] = [ES256, EDDSA, RS256];

// COSE_Key labels and values (RFC 9052, RFC 9053, RFC 8230).
const KTY: i64 = 1;
const ALG: i64 = 3;
const KTY_OKP: i64 = 1;
const KTY_EC2: i64 = 2;
const KTY_RSA: i64 = 3;
const CRV: i64 = -1;
const X: i64 = -2;
const Y: i64 = -3;
const RSA_N: i64 = -1;
const RSA_E: i64 = -2;
const CRV_P256: i64 = 1;
const CRV_ED25519: i64 = 6;

/// A passkey's public key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoseKey {
    /// P-256, as an uncompressed point (`0x04 ‖ x ‖ y`).
    Es256([u8; 65]),
    /// Ed25519.
    Ed25519([u8; 32]),
    /// RSA, 2048 to 8192 bits: modulus and exponent, big-endian.
    Rs256 {
        /// The modulus.
        n: Vec<u8>,
        /// The public exponent.
        e: Vec<u8>,
    },
}

impl CoseKey {
    /// Reads a key stored as COSE bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, WebauthnError> {
        let value = cbor::decode(bytes).map_err(|_| WebauthnError("public key isn't CBOR"))?;
        Self::from_value(&value)
    }

    /// Reads a key from a decoded COSE_Key map.
    pub fn from_value(key: &Value<'_>) -> Result<Self, WebauthnError> {
        let int = |label| key.get_int(label).and_then(Value::as_int);
        let bytes = |label| key.get_int(label).and_then(Value::as_bytes);
        let unsupported = WebauthnError("unsupported public key");
        match (int(KTY), int(ALG)) {
            (Some(KTY_EC2), Some(alg)) if alg == i64::from(ES256) => {
                let (Some(x), Some(y)) = (bytes(X), bytes(Y)) else {
                    return Err(unsupported);
                };
                if int(CRV) != Some(CRV_P256) || x.len() != 32 || y.len() != 32 {
                    return Err(unsupported);
                }
                let mut point = [0u8; 65];
                point[0] = 0x04;
                point[1..33].copy_from_slice(x);
                point[33..].copy_from_slice(y);
                Ok(Self::Es256(point))
            }
            (Some(KTY_OKP), Some(alg)) if alg == i64::from(EDDSA) => {
                let x = bytes(X).filter(|_| int(CRV) == Some(CRV_ED25519));
                let x: [u8; 32] = x.and_then(|x| x.try_into().ok()).ok_or(unsupported)?;
                Ok(Self::Ed25519(x))
            }
            (Some(KTY_RSA), Some(alg)) if alg == i64::from(RS256) => {
                let (Some(n), Some(e)) = (bytes(RSA_N), bytes(RSA_E)) else {
                    return Err(unsupported);
                };
                if !(256..=1024).contains(&n.len()) || e.is_empty() || e.len() > 8 {
                    return Err(unsupported);
                }
                Ok(Self::Rs256 {
                    n: n.to_vec(),
                    e: e.to_vec(),
                })
            }
            _ => Err(unsupported),
        }
    }

    /// The key's COSE algorithm.
    pub fn algorithm(&self) -> i32 {
        match self {
            Self::Es256(_) => ES256,
            Self::Ed25519(_) => EDDSA,
            Self::Rs256 { .. } => RS256,
        }
    }

    /// Checks that `signature` signs `message` with this key.
    pub fn verify(&self, message: &[u8], signature: &[u8]) -> Result<(), WebauthnError> {
        let checked = match self {
            Self::Es256(point) => UnparsedPublicKey::new(&signature::ECDSA_P256_SHA256_ASN1, point)
                .verify(message, signature),
            Self::Ed25519(x) => {
                UnparsedPublicKey::new(&signature::ED25519, x).verify(message, signature)
            }
            Self::Rs256 { n, e } => RsaPublicKeyComponents { n, e }.verify(
                &signature::RSA_PKCS1_2048_8192_SHA256,
                message,
                signature,
            ),
        };
        checked.map_err(|_| WebauthnError("signature doesn't match the passkey"))
    }
}
