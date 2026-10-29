//! Passkey checks (WebAuthn Level 3, §7.1 registration and §7.2 authentication), written on
//! `ring` for the signatures and a small CBOR reader, with no WebAuthn crate (the ones on
//! crates.io pull in OpenSSL).
//!
//! What is checked, every time: the challenge the server issued, the page's origin (top-level
//! only), the RP ID hash, user presence **and** user verification (biometrics or PIN), and the
//! backup flags. On sign-in, also the signature, the user handle and the signature counter.
//!
//! Attestation is not verified: the server asks for none (`attestation: "none"`) and accepts
//! any format, reading only the authenticator data. Nothing here trusts the authenticator's
//! make; a new passkey is trusted because a signed-in user registered it.

mod authenticator_data;
pub mod cbor;
mod client_data;
pub mod cose;

pub use authenticator_data::{
    ATTESTED, AttestedCredential, AuthenticatorData, BACKED_UP, BACKUP_ELIGIBLE, EXTENSIONS,
    USER_PRESENT, USER_VERIFIED, parse as parse_authenticator_data,
};
pub use cose::CoseKey;

use crate::shared::crypto::sha256;

/// Why a ceremony's response was refused. The reason is for the logs; callers answer every
/// refusal with the same error code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("passkey refused: {0}")]
pub struct WebauthnError(pub &'static str);

/// What the server expects of a ceremony.
#[derive(Debug, Clone, Copy)]
pub struct Expected<'a> {
    /// The challenge it issued.
    pub challenge: &'a [u8],
    /// The web page's origin, e.g. `https://app.paccradle.com`.
    pub origin: &'a str,
    /// The relying party ID passkeys are bound to, e.g. `paccradle.com`.
    pub rp_id: &'a str,
}

/// A browser's answer to `navigator.credentials.create()`, decoded from base64url.
#[derive(Debug, Clone, Copy)]
pub struct Registration<'a> {
    /// `response.clientDataJSON`.
    pub client_data_json: &'a [u8],
    /// `response.attestationObject`.
    pub attestation_object: &'a [u8],
}

/// A passkey that passed registration, ready to store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewCredential {
    /// The credential id.
    pub credential_id: Vec<u8>,
    /// The COSE public key, as sent.
    pub public_key: Vec<u8>,
    /// Its COSE algorithm.
    pub algorithm: i32,
    /// The signature counter at creation.
    pub sign_count: u32,
    /// Whether it can be synced (fixed for its life).
    pub backup_eligible: bool,
    /// Whether it is synced now.
    pub backed_up: bool,
    /// The authenticator model, unless it doesn't say.
    pub aaguid: Option<[u8; 16]>,
}

/// A browser's answer to `navigator.credentials.get()`, decoded from base64url.
#[derive(Debug, Clone, Copy)]
pub struct Authentication<'a> {
    /// `response.clientDataJSON`.
    pub client_data_json: &'a [u8],
    /// `response.authenticatorData`.
    pub authenticator_data: &'a [u8],
    /// `response.signature`.
    pub signature: &'a [u8],
    /// `response.userHandle`: whose passkey the authenticator says it is.
    pub user_handle: Option<&'a [u8]>,
}

/// What the server knows of the passkey being used.
#[derive(Debug, Clone, Copy)]
pub struct StoredCredential<'a> {
    /// The COSE public key stored at registration.
    pub public_key: &'a [u8],
    /// The last signature counter seen.
    pub sign_count: u32,
    /// Its backup eligibility, fixed at registration.
    pub backup_eligible: bool,
    /// The user handle it was made for (the user's id).
    pub user_handle: &'a [u8],
}

/// What a sign-in tells about the passkey, to store.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Assertion {
    /// The new signature counter.
    pub sign_count: u32,
    /// Whether it is synced now.
    pub backed_up: bool,
}

/// Checks a new passkey (WebAuthn §7.1) and returns what to store.
pub fn verify_registration(
    expected: &Expected<'_>,
    response: &Registration<'_>,
) -> Result<NewCredential, WebauthnError> {
    client_data::check(response.client_data_json, (client_data::CREATE, expected))?;
    let attestation = cbor::decode(response.attestation_object)
        .map_err(|_| WebauthnError("attestation object isn't CBOR"))?;
    let format = attestation.get_text("fmt").and_then(cbor::Value::as_text);
    let statement = attestation.get_text("attStmt");
    let auth_data = attestation
        .get_text("authData")
        .and_then(cbor::Value::as_bytes);
    let (Some(_), Some(cbor::Value::Map(_)), Some(auth_data)) = (format, statement, auth_data)
    else {
        return Err(WebauthnError(
            "attestation object lacks fmt, attStmt or authData",
        ));
    };

    let data = authenticator_data::parse(auth_data)?;
    check_authenticator(&data, expected)?;
    let Some(credential) = &data.credential else {
        return Err(WebauthnError("no new credential in the authenticator data"));
    };
    if !(16..=1023).contains(&credential.credential_id.len()) {
        return Err(WebauthnError("credential id has an odd length"));
    }
    let key = CoseKey::from_bytes(credential.public_key)?;
    Ok(NewCredential {
        credential_id: credential.credential_id.to_vec(),
        public_key: credential.public_key.to_vec(),
        algorithm: key.algorithm(),
        sign_count: data.sign_count,
        backup_eligible: data.has(BACKUP_ELIGIBLE),
        backed_up: data.has(BACKED_UP),
        aaguid: Some(credential.aaguid).filter(|aaguid| aaguid.iter().any(|&b| b != 0)),
    })
}

/// Checks a sign-in with a stored passkey (WebAuthn §7.2) and returns its new state.
pub fn verify_authentication(
    expected: &Expected<'_>,
    (stored, response): (&StoredCredential<'_>, &Authentication<'_>),
) -> Result<Assertion, WebauthnError> {
    // The sign-in names no user up front (discoverable passkeys), so the handle must be there.
    if response.user_handle != Some(stored.user_handle) {
        return Err(WebauthnError(
            "user handle doesn't match the passkey's owner",
        ));
    }
    client_data::check(response.client_data_json, (client_data::GET, expected))?;
    let data = authenticator_data::parse(response.authenticator_data)?;
    check_authenticator(&data, expected)?;
    if data.has(BACKUP_ELIGIBLE) != stored.backup_eligible {
        return Err(WebauthnError("backup eligibility changed"));
    }

    let mut message = Vec::with_capacity(response.authenticator_data.len() + 32);
    message.extend_from_slice(response.authenticator_data);
    message.extend_from_slice(&sha256(response.client_data_json));
    CoseKey::from_bytes(stored.public_key)?.verify(&message, response.signature)?;

    // A counter that doesn't move forward may mean a cloned authenticator. Synced passkeys
    // keep it at 0, which is fine as long as it stays there.
    let counting = data.sign_count != 0 || stored.sign_count != 0;
    if counting && data.sign_count <= stored.sign_count {
        return Err(WebauthnError("signature counter didn't increase"));
    }
    Ok(Assertion {
        sign_count: data.sign_count,
        backed_up: data.has(BACKED_UP),
    })
}

/// The checks both ceremonies make on the authenticator data.
fn check_authenticator(
    data: &AuthenticatorData<'_>,
    expected: &Expected<'_>,
) -> Result<(), WebauthnError> {
    if data.rp_id_hash != sha256(expected.rp_id.as_bytes()) {
        return Err(WebauthnError("passkey is for another site (RP ID hash)"));
    }
    if !data.has(USER_PRESENT) {
        return Err(WebauthnError("user wasn't present"));
    }
    if !data.has(USER_VERIFIED) {
        return Err(WebauthnError("user wasn't verified"));
    }
    if data.has(BACKED_UP) && !data.has(BACKUP_ELIGIBLE) {
        return Err(WebauthnError("backed up but not backup eligible"));
    }
    Ok(())
}
