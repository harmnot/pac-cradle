//! A software passkey authenticator for tests: it makes credentials and signs like a real one
//! (P-256, Ed25519 or RSA keys from `ring`), writing CBOR with a few helpers. It answers the
//! API's W3C options JSON with the W3C response JSON, as `credential.toJSON()` would.

use paccradle_backend::shared::base64url;
use paccradle_backend::shared::crypto::sha256;
use ring::rand::SystemRandom;
use ring::signature::{
    ECDSA_P256_SHA256_ASN1_SIGNING, EcdsaKeyPair, Ed25519KeyPair, KeyPair, RSA_PKCS1_SHA256,
    RsaKeyPair,
};
use serde_json::{Value, json};

/// The flags a synced platform passkey sets: user present and verified, backup eligible and
/// backed up.
pub const PASSKEY_FLAGS: u8 = 0x01 | 0x04 | 0x08 | 0x10;
/// Attested credential data follows.
pub const ATTESTED: u8 = 0x40;

/// A 2048-bit RSA key made with OpenSSL for these tests only (PKCS#8, base64url).
const RSA_PKCS8: &str = "MIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQCJlwGSKT_NlKBBNgPsciwDjPMASXWpnw5udNq4\
    PgPlrT2kVmArPnBWWF36wnqHBZ_2LfedIte4GjJpFurf_VvXw7aX_pwBqPIWD8zFK7HhsS1l07TDE9ZjACuVFoFI\
    D04yfwQDZw03HBYU-GDxvMNMSifteS-\
    4nvAr8tQgAdNJK38h1p9sNDDGmlTB5HXKoIR1Bbo8MES8RwEG0wdXpnlSA7MAYau1u6I5wz9IYh6KxGipEizestD\
    KW9GaX6804XX2oGS_KXFVHLfzoYnZ6gyxrcf9dTVdSJzSBFa1LQ0DfjJxYDE4_qG-\
    N2RI6aK6mrJneu04e0reD4Bbp67b6TxrAgMBAAECggEAAqSnABLQyfc_-OC7xJAQ7mSHyoqeuwp2MYMHLKDsXD4u\
    v_h00n_Zkp8R9x5DUlK2VvldF2Sy9jNyongxPOpWGFgHKvZWMuX2-JuHGIaVXTuVHq6pe1HnPXGkZLHQ-_xnSSsN\
    fa8-\
    Hr_oYLDA1AQHUHDgQXyhgQ90l7OLha3jTDWFfrcUcdHh1SMWSB9eLsCp_NMDvjhDlYdSXgT3_DQ8h36LeIZdHVbR\
    rjUjxPosUKNeuLPM3VQ91Exi90-\
    UlvqQDwwTgGfr93efDsC4xJ3HbW3c7ZXiumikubuPx_rf1zlvsanAvb2K0lBfyjjJ2tcgZWumYkhGy9-\
    vUAiDcdCXgQKBgQC8MqiR8dlfGFQTOvVme69BoQMtvOg6HKAomc3v7eg2fMOfT9w3I1f1IXLVPFuaG0P4MMDYarH\
    xNL8UjVWjYvq9rO3SoMg1zB_EAXcSXXakRfWpxcbABXEgS2rdfggm2yzzkh6CHKqt-TJqm5M2xgL0C-\
    l0uxvOnOqJTkGtMNeGQQKBgQC7KM94C-\
    B7QlHCzmBBggLM_1HwFYuWw6lOfqRNdx5KNTW2wDY6RlrtwBpU4FHlQwkCCzk4kd9Kber4FHOZBlS3jcmjtft9Ii\
    8R9q8jpJmgS7yEz5yAQ269uFvCGlco1OSuR64u6J96nE_8MwPaCSIq0-\
    8Lg_LjULQTX5ThRWTPqwKBgQCZF5yJHHP3J5M9EXkPJQ4sFQ7A5isDGsYuQO-FfcKokFWSAyTgtZ-\
    XiN4hGDhr5EImY4J8ik62KHJIBbDkeGAAa-wKq0AygxpUF90E04Lvnvl4YaqMCeE8pB0KkLuQ4Dpw6SxoK2msnEq\
    m7meS5wEABbCjhJHaQUuI5mAGwvSQAQKBgH4ycGJDr-OsvjCas3C4V2ceqYM3I95OpDSXU_Yf4b6rzkIhrO7yTZG\
    OPLMKZm4uGz7TjovEYElxgEQylheSm0EBS_slVCP2DsMQENv5utTI_J6Y1q_tyBgI3WYuGKMhmeih0KM0m4QzLgO\
    h6nD0Q6OZL9AV8oqv-hwuENcs0WLtAoGAH9VIXnbVKmc6WxKLN2Fhmo9mZEQe9W0CpdsSWOLp9CvObj2LI3enXW4\
    u1m7BzPW_3kOan6hkX20yDxb5w6ml-\
    lTfyZJX0ZA78zYS4fZJgqeFLqgkWeeR_5lp3Ef7EeZqgMN3gioPz7bin7_rJ79GCDtENMnb7-yHnwGwG-rsaEA";
/// Its modulus, hex. The exponent is 65537.
const RSA_MODULUS: &str = "89970192293FCD94A0413603EC722C038CF3004975A99F0E6E74DAB83E03E5AD3DA456602B3E7056585DFAC2\
    7A87059FF62DF79D22D7B81A326916EADFFD5BD7C3B697FE9C01A8F2160FCCC52BB1E1B12D65D3B4C313D663\
    002B951681480F4E327F0403670D371C1614F860F1BCC34C4A27ED792FB89EF02BF2D42001D3492B7F21D69F\
    6C3430C69A54C1E475CAA0847505BA3C3044BC470106D30757A6795203B30061ABB5BBA239C33F48621E8AC4\
    68A9122CDEB2D0CA5BD19A5FAF34E175F6A064BF2971551CB7F3A189D9EA0CB1ADC7FD75355D489CD20456B5\
    2D0D037E3271603138FEA1BE376448E9A2BA9AB2677AED387B4ADE0F805BA7AEDBE93C6B";

/// Which key the authenticator makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyKind {
    /// ES256.
    P256,
    /// EdDSA.
    Ed25519,
    /// RS256.
    Rsa,
}

enum Signer {
    P256(EcdsaKeyPair),
    Ed25519(Ed25519KeyPair),
    Rsa(RsaKeyPair),
}

/// One credential on a pretend device.
pub struct SoftAuthenticator {
    signer: Signer,
    /// The credential id.
    pub credential_id: Vec<u8>,
    /// The signature counter; it goes up before each signature when `counting`.
    pub sign_count: u32,
    /// Whether the counter moves (synced passkeys keep it at 0).
    pub counting: bool,
    /// The RP ID it signs for.
    pub rp_id: String,
    /// The page origin its client data names.
    pub origin: String,
    /// The flag byte (without the attested bit, added on registration).
    pub flags: u8,
    /// The user handle registration gave it.
    pub user_handle: Option<Vec<u8>>,
}

impl SoftAuthenticator {
    /// A new credential for RP ID `localhost`, used from `origin`.
    pub fn new(kind: KeyKind, origin: &str) -> Self {
        let rng = SystemRandom::new();
        let signer = match kind {
            KeyKind::P256 => {
                let pkcs8 =
                    EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, &rng).unwrap();
                Signer::P256(
                    EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, pkcs8.as_ref(), &rng)
                        .unwrap(),
                )
            }
            KeyKind::Ed25519 => {
                let pkcs8 = Ed25519KeyPair::generate_pkcs8(&rng).unwrap();
                Signer::Ed25519(Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).unwrap())
            }
            KeyKind::Rsa => {
                Signer::Rsa(RsaKeyPair::from_pkcs8(&base64url::decode(RSA_PKCS8).unwrap()).unwrap())
            }
        };
        let mut credential_id = vec![0u8; 32];
        ring::rand::SecureRandom::fill(&rng, &mut credential_id).unwrap();
        Self {
            signer,
            credential_id,
            sign_count: 0,
            counting: false,
            rp_id: "localhost".to_owned(),
            origin: origin.to_owned(),
            flags: PASSKEY_FLAGS,
            user_handle: None,
        }
    }

    /// The public key as a COSE_Key.
    pub fn cose_key(&self) -> Vec<u8> {
        match &self.signer {
            Signer::P256(key) => {
                let point = key.public_key().as_ref();
                cbor_map(&[
                    (cbor_int(1), cbor_int(2)),
                    (cbor_int(3), cbor_int(-7)),
                    (cbor_int(-1), cbor_int(1)),
                    (cbor_int(-2), cbor_bytes(&point[1..33])),
                    (cbor_int(-3), cbor_bytes(&point[33..])),
                ])
            }
            Signer::Ed25519(key) => cbor_map(&[
                (cbor_int(1), cbor_int(1)),
                (cbor_int(3), cbor_int(-8)),
                (cbor_int(-1), cbor_int(6)),
                (cbor_int(-2), cbor_bytes(key.public_key().as_ref())),
            ]),
            Signer::Rsa(_) => cbor_map(&[
                (cbor_int(1), cbor_int(3)),
                (cbor_int(3), cbor_int(-257)),
                (cbor_int(-1), cbor_bytes(&hex(RSA_MODULUS))),
                (cbor_int(-2), cbor_bytes(&[1, 0, 1])),
            ]),
        }
    }

    /// Signs `message` the way the key's algorithm does.
    pub fn sign(&self, message: &[u8]) -> Vec<u8> {
        let rng = SystemRandom::new();
        match &self.signer {
            Signer::P256(key) => key.sign(&rng, message).unwrap().as_ref().to_vec(),
            Signer::Ed25519(key) => key.sign(message).as_ref().to_vec(),
            Signer::Rsa(key) => {
                let mut signature = vec![0u8; key.public().modulus_len()];
                key.sign(&RSA_PKCS1_SHA256, &rng, message, &mut signature)
                    .unwrap();
                signature
            }
        }
    }

    /// Authenticator data: RP ID hash, flags, counter, and the new credential when `attested`.
    pub fn authenticator_data(&self, attested: bool) -> Vec<u8> {
        let mut data = sha256(self.rp_id.as_bytes()).to_vec();
        data.push(if attested {
            self.flags | ATTESTED
        } else {
            self.flags
        });
        data.extend_from_slice(&self.sign_count.to_be_bytes());
        if attested {
            data.extend_from_slice(&[0u8; 16]);
            data.extend_from_slice(
                &u16::try_from(self.credential_id.len())
                    .unwrap()
                    .to_be_bytes(),
            );
            data.extend_from_slice(&self.credential_id);
            data.extend_from_slice(&self.cose_key());
        }
        data
    }

    /// Client data JSON for a `kind` ceremony with `challenge`, from this authenticator's origin.
    pub fn client_data(&self, kind: &str, challenge: &[u8]) -> Vec<u8> {
        serde_json::to_vec(&json!({
            "type": kind,
            "challenge": base64url::encode(challenge),
            "origin": self.origin,
            "crossOrigin": false,
        }))
        .unwrap()
    }

    /// `(clientDataJSON, attestationObject)` for a registration with `challenge`.
    pub fn registration_parts(&self, challenge: &[u8]) -> (Vec<u8>, Vec<u8>) {
        let client_data = self.client_data("webauthn.create", challenge);
        let attestation = cbor_map(&[
            (cbor_text("fmt"), cbor_text("none")),
            (cbor_text("attStmt"), cbor_map(&[])),
            (
                cbor_text("authData"),
                cbor_bytes(&self.authenticator_data(true)),
            ),
        ]);
        (client_data, attestation)
    }

    /// `(clientDataJSON, authenticatorData, signature)` for a sign-in with `challenge`.
    pub fn assertion_parts(&mut self, challenge: &[u8]) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
        if self.counting {
            self.sign_count += 1;
        }
        let client_data = self.client_data("webauthn.get", challenge);
        let auth_data = self.authenticator_data(false);
        let mut message = auth_data.clone();
        message.extend_from_slice(&sha256(&client_data));
        let signature = self.sign(&message);
        (client_data, auth_data, signature)
    }

    /// `RegistrationResponseJSON` for the API's creation options (`public_key`).
    pub fn register(&mut self, public_key: &Value) -> Value {
        let challenge = base64url::decode(public_key["challenge"].as_str().unwrap()).unwrap();
        self.user_handle =
            Some(base64url::decode(public_key["user"]["id"].as_str().unwrap()).unwrap());
        let (client_data, attestation) = self.registration_parts(&challenge);
        let id = base64url::encode(&self.credential_id);
        json!({
            "id": id,
            "rawId": id,
            "type": "public-key",
            "authenticatorAttachment": "platform",
            "clientExtensionResults": { "credProps": { "rk": true } },
            "response": {
                "clientDataJSON": base64url::encode(&client_data),
                "attestationObject": base64url::encode(&attestation),
                "transports": ["internal", "hybrid", "carrier-pigeon"],
            },
        })
    }

    /// `AuthenticationResponseJSON` for the API's request options (`public_key`).
    pub fn sign_in(&mut self, public_key: &Value) -> Value {
        let challenge = base64url::decode(public_key["challenge"].as_str().unwrap()).unwrap();
        let (client_data, auth_data, signature) = self.assertion_parts(&challenge);
        let id = base64url::encode(&self.credential_id);
        json!({
            "id": id,
            "rawId": id,
            "type": "public-key",
            "clientExtensionResults": {},
            "response": {
                "clientDataJSON": base64url::encode(&client_data),
                "authenticatorData": base64url::encode(&auth_data),
                "signature": base64url::encode(&signature),
                "userHandle": self.user_handle.as_deref().map(base64url::encode),
            },
        })
    }
}

fn cbor_head(major: u8, n: u64) -> Vec<u8> {
    let major = major << 5;
    match n {
        0..=23 => vec![major | n as u8],
        24..=0xff => vec![major | 24, n as u8],
        0x100..=0xffff => [vec![major | 25], (n as u16).to_be_bytes().to_vec()].concat(),
        0x1_0000..=0xffff_ffff => [vec![major | 26], (n as u32).to_be_bytes().to_vec()].concat(),
        _ => [vec![major | 27], n.to_be_bytes().to_vec()].concat(),
    }
}

/// A CBOR integer.
pub fn cbor_int(n: i64) -> Vec<u8> {
    match u64::try_from(n) {
        Ok(n) => cbor_head(0, n),
        Err(_) => cbor_head(1, (-1 - n) as u64),
    }
}

/// A CBOR byte string.
pub fn cbor_bytes(bytes: &[u8]) -> Vec<u8> {
    [cbor_head(2, bytes.len() as u64), bytes.to_vec()].concat()
}

/// A CBOR text string.
pub fn cbor_text(text: &str) -> Vec<u8> {
    [cbor_head(3, text.len() as u64), text.as_bytes().to_vec()].concat()
}

/// A CBOR map of already-encoded keys and values.
pub fn cbor_map(pairs: &[(Vec<u8>, Vec<u8>)]) -> Vec<u8> {
    let mut out = cbor_head(5, pairs.len() as u64);
    for (key, value) in pairs {
        out.extend_from_slice(key);
        out.extend_from_slice(value);
    }
    out
}

/// A CBOR array of already-encoded items.
pub fn cbor_array(items: &[Vec<u8>]) -> Vec<u8> {
    let mut out = cbor_head(4, items.len() as u64);
    for item in items {
        out.extend_from_slice(item);
    }
    out
}

fn hex(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
        .collect()
}
