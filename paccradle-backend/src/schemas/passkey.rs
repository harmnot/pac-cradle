//! Passkey bodies (M1-D3). The `public_key` options and the `credential` answers use the W3C
//! WebAuthn JSON shapes (camelCase, base64url), so the web app can pass them straight to
//! `PublicKeyCredential.parseCreationOptionsFromJSON` / `parseRequestOptionsFromJSON` and send
//! back `credential.toJSON()`.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use super::identity::invalid;
use crate::error::AppError;
use crate::models::passkey::{
    AuthenticationCredential, Passkey, PasskeyOptions, PasskeyOptionsKind, RegistrationCredential,
};
use crate::shared::base64url;
use crate::shared::webauthn::cose::ALGORITHMS;

/// The `type` of every credential.
const PUBLIC_KEY: &str = "public-key";
/// The transports a browser may report; others are dropped.
const KNOWN_TRANSPORTS: [&str; 6] = ["ble", "hybrid", "internal", "nfc", "smart-card", "usb"];
/// The longest passkey name, in characters.
const MAX_NAME_CHARS: usize = 100;

/// Options for `navigator.credentials.create()` or `.get()`, and the ceremony to answer.
#[derive(Debug, Serialize)]
pub struct PasskeyOptionsResponse {
    /// Send it back with the answer.
    pub ceremony_id: Uuid,
    /// The W3C options JSON.
    pub public_key: PublicKeyOptions,
}

/// `PublicKeyCredentialCreationOptionsJSON` or `PublicKeyCredentialRequestOptionsJSON`.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum PublicKeyOptions {
    /// Registration.
    Creation(CreationOptions),
    /// Sign-in.
    Request(RequestOptions),
}

/// `PublicKeyCredentialCreationOptionsJSON`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreationOptions {
    rp: RelyingParty,
    user: UserEntity,
    challenge: String,
    pub_key_cred_params: Vec<CredentialParameter>,
    timeout: u64,
    exclude_credentials: Vec<CredentialDescriptor>,
    authenticator_selection: AuthenticatorSelection,
    attestation: &'static str,
    extensions: CreationExtensions,
}

/// `PublicKeyCredentialRequestOptionsJSON`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestOptions {
    challenge: String,
    rp_id: String,
    timeout: u64,
    user_verification: &'static str,
    allow_credentials: Vec<CredentialDescriptor>,
}

#[derive(Debug, Serialize)]
struct RelyingParty {
    id: String,
    name: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UserEntity {
    id: String,
    name: String,
    display_name: String,
}

#[derive(Debug, Serialize)]
struct CredentialParameter {
    #[serde(rename = "type")]
    kind: &'static str,
    alg: i32,
}

#[derive(Debug, Serialize)]
struct CredentialDescriptor {
    #[serde(rename = "type")]
    kind: &'static str,
    id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AuthenticatorSelection {
    resident_key: &'static str,
    require_resident_key: bool,
    user_verification: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CreationExtensions {
    cred_props: bool,
}

impl From<PasskeyOptions> for PasskeyOptionsResponse {
    fn from(options: PasskeyOptions) -> Self {
        let challenge = base64url::encode(&options.challenge);
        let timeout = u64::try_from(options.timeout.as_millis()).unwrap_or(u64::MAX);
        let public_key = match options.kind {
            PasskeyOptionsKind::SignIn => PublicKeyOptions::Request(RequestOptions {
                challenge,
                rp_id: options.rp_id,
                timeout,
                user_verification: "required",
                // Empty: the browser offers every passkey it has for the site.
                allow_credentials: Vec::new(),
            }),
            PasskeyOptionsKind::Register {
                rp_name,
                user_id,
                user_name,
                exclude,
            } => PublicKeyOptions::Creation(CreationOptions {
                rp: RelyingParty {
                    id: options.rp_id,
                    name: rp_name,
                },
                user: UserEntity {
                    id: base64url::encode(user_id.as_bytes()),
                    display_name: user_name.clone(),
                    name: user_name,
                },
                challenge,
                pub_key_cred_params: ALGORITHMS
                    .iter()
                    .map(|&alg| CredentialParameter {
                        kind: PUBLIC_KEY,
                        alg,
                    })
                    .collect(),
                timeout,
                exclude_credentials: exclude
                    .iter()
                    .map(|id| CredentialDescriptor {
                        kind: PUBLIC_KEY,
                        id: base64url::encode(id),
                    })
                    .collect(),
                authenticator_selection: AuthenticatorSelection {
                    resident_key: "required",
                    require_resident_key: true,
                    user_verification: "required",
                },
                attestation: "none",
                extensions: CreationExtensions { cred_props: true },
            }),
        };
        Self {
            ceremony_id: options.ceremony_id,
            public_key,
        }
    }
}

/// `POST /auth/passkey/sign-in`.
#[derive(Deserialize)]
pub struct PasskeySignInRequest {
    /// From the options.
    pub ceremony_id: Uuid,
    /// `AuthenticationResponseJSON` (`credential.toJSON()`).
    pub credential: AuthenticationResponseJson,
}

/// `AuthenticationResponseJSON`. Other members (`authenticatorAttachment`,
/// `clientExtensionResults`) are ignored.
#[derive(Deserialize)]
pub struct AuthenticationResponseJson {
    id: String,
    #[serde(rename = "rawId")]
    raw_id: String,
    #[serde(rename = "type")]
    kind: String,
    response: AssertionResponseJson,
}

#[derive(Deserialize)]
struct AssertionResponseJson {
    #[serde(rename = "clientDataJSON")]
    client_data_json: String,
    #[serde(rename = "authenticatorData")]
    authenticator_data: String,
    signature: String,
    #[serde(rename = "userHandle", default)]
    user_handle: Option<String>,
}

impl AuthenticationResponseJson {
    /// The answer with its fields decoded.
    pub fn decode(&self) -> Result<AuthenticationCredential, AppError> {
        let raw_id = credential_id((&self.id, &self.raw_id, &self.kind))?;
        let response = &self.response;
        Ok(AuthenticationCredential {
            raw_id,
            client_data_json: field(&response.client_data_json, "response.clientDataJSON")?,
            authenticator_data: field(&response.authenticator_data, "response.authenticatorData")?,
            signature: field(&response.signature, "response.signature")?,
            user_handle: response
                .user_handle
                .as_deref()
                .map(|handle| field(handle, "response.userHandle"))
                .transpose()?,
        })
    }
}

/// `POST /me/passkeys`.
#[derive(Deserialize)]
pub struct PasskeyRegistrationRequest {
    /// From the options.
    pub ceremony_id: Uuid,
    /// `RegistrationResponseJSON` (`credential.toJSON()`).
    pub credential: RegistrationResponseJson,
    /// A name to tell passkeys apart, e.g. "MacBook" (at most 100 characters).
    #[serde(default)]
    pub name: Option<String>,
}

impl PasskeyRegistrationRequest {
    /// The name, trimmed (empty when not given).
    pub fn name(&self) -> Result<String, AppError> {
        let name = self.name.as_deref().unwrap_or_default().trim();
        if name.chars().count() > MAX_NAME_CHARS || name.chars().any(char::is_control) {
            return Err(invalid("name"));
        }
        Ok(name.to_owned())
    }
}

/// `RegistrationResponseJSON`. Other members are ignored.
#[derive(Deserialize)]
pub struct RegistrationResponseJson {
    id: String,
    #[serde(rename = "rawId")]
    raw_id: String,
    #[serde(rename = "type")]
    kind: String,
    response: AttestationResponseJson,
}

#[derive(Deserialize)]
struct AttestationResponseJson {
    #[serde(rename = "clientDataJSON")]
    client_data_json: String,
    #[serde(rename = "attestationObject")]
    attestation_object: String,
    #[serde(default)]
    transports: Vec<String>,
}

impl RegistrationResponseJson {
    /// The answer with its fields decoded.
    pub fn decode(&self) -> Result<RegistrationCredential, AppError> {
        credential_id((&self.id, &self.raw_id, &self.kind))?;
        let response = &self.response;
        let mut transports: Vec<String> = response
            .transports
            .iter()
            .filter(|transport| KNOWN_TRANSPORTS.contains(&transport.as_str()))
            .cloned()
            .collect();
        transports.sort_unstable();
        transports.dedup();
        Ok(RegistrationCredential {
            client_data_json: field(&response.client_data_json, "response.clientDataJSON")?,
            attestation_object: field(&response.attestation_object, "response.attestationObject")?,
            transports,
        })
    }
}

/// A stored passkey, as listed to its owner.
#[derive(Debug, Serialize)]
pub struct PasskeyResponse {
    /// Our id for it.
    pub id: Uuid,
    /// Its name (may be empty).
    pub name: String,
    /// Whether it is synced to the user's other devices.
    pub backed_up: bool,
    /// When it was added, RFC 3339.
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

impl From<Passkey> for PasskeyResponse {
    fn from(passkey: Passkey) -> Self {
        Self {
            id: passkey.id,
            name: passkey.name,
            backed_up: passkey.backed_up,
            created_at: passkey.created_at,
        }
    }
}

/// The decoded credential id, once `id` and `rawId` agree and `type` is `public-key`.
fn credential_id((id, raw_id, kind): (&str, &str, &str)) -> Result<Vec<u8>, AppError> {
    if kind != PUBLIC_KEY {
        return Err(invalid("credential.type"));
    }
    if id != raw_id {
        return Err(invalid("credential.rawId"));
    }
    field(raw_id, "rawId")
}

/// A base64url member of `credential`, decoded.
fn field(value: &str, name: &str) -> Result<Vec<u8>, AppError> {
    base64url::decode(value).ok_or_else(|| invalid(&format!("credential.{name}")))
}
