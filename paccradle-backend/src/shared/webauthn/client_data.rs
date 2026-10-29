//! Client data (WebAuthn §5.8.1): the JSON the browser builds and the authenticator signs a
//! hash of. It carries the ceremony type, the challenge and the page's origin.

use serde::Deserialize;

use super::{Expected, WebauthnError};
use crate::shared::base64url;

/// `type` when creating a passkey.
pub const CREATE: &str = "webauthn.create";
/// `type` when signing in with one.
pub const GET: &str = "webauthn.get";

#[derive(Deserialize)]
struct ClientData {
    #[serde(rename = "type")]
    kind: String,
    challenge: String,
    origin: String,
    #[serde(rename = "crossOrigin", default)]
    cross_origin: Option<bool>,
    #[serde(rename = "topOrigin", default)]
    top_origin: Option<String>,
}

/// Checks `json` is client data for a `kind` ceremony with the expected challenge, made by a
/// top-level page on the expected origin (never inside another site's frame).
pub fn check(json: &[u8], (kind, expected): (&str, &Expected<'_>)) -> Result<(), WebauthnError> {
    let data: ClientData =
        serde_json::from_slice(json).map_err(|_| WebauthnError("client data isn't readable"))?;
    if data.kind != kind {
        return Err(WebauthnError("client data is for another ceremony"));
    }
    if base64url::decode(&data.challenge).as_deref() != Some(expected.challenge) {
        return Err(WebauthnError("client data has another challenge"));
    }
    if data.origin != expected.origin {
        return Err(WebauthnError("client data comes from another origin"));
    }
    if data.cross_origin == Some(true) || data.top_origin.is_some() {
        return Err(WebauthnError("client data comes from a frame"));
    }
    Ok(())
}
