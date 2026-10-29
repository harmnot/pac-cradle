//! Passkeys (the `passkeys` table) and their ceremonies (`passkey_challenges`).

use std::time::Duration;

use time::OffsetDateTime;
use uuid::Uuid;

use super::TextEnum;
use crate::shared::api_surface::ApiSurface;
use crate::shared::webauthn::NewCredential;

/// A stored passkey.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Passkey {
    /// Our id for it.
    pub id: Uuid,
    /// Whose it is.
    pub user_id: Uuid,
    /// The authenticator's credential id.
    pub credential_id: Vec<u8>,
    /// The COSE public key.
    pub public_key: Vec<u8>,
    /// The last signature counter seen.
    pub sign_count: u32,
    /// Whether it can be synced (fixed).
    pub backup_eligible: bool,
    /// Whether it was synced when last seen.
    pub backed_up: bool,
    /// The name the user gave it (may be empty).
    pub name: String,
    /// When it was added.
    pub created_at: OffsetDateTime,
}

/// A passkey to add.
#[derive(Debug, Clone)]
pub struct NewPasskey {
    /// The id it will have.
    pub id: Uuid,
    /// Whose it is.
    pub user_id: Uuid,
    /// What registration checked.
    pub credential: NewCredential,
    /// How the browser can reach the authenticator.
    pub transports: Vec<String>,
    /// Its name.
    pub name: String,
}

/// What a sign-in changed about a passkey.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PasskeyUse {
    /// The new signature counter.
    pub sign_count: u32,
    /// Whether it is synced now.
    pub backed_up: bool,
}

/// `passkey_challenges.purpose`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChallengePurpose {
    /// Sign in with any passkey.
    SignIn,
    /// Add a passkey to a signed-in user.
    Register,
}

impl TextEnum for ChallengePurpose {
    const ALL: &'static [Self] = &[Self::SignIn, Self::Register];

    fn as_str(self) -> &'static str {
        match self {
            Self::SignIn => "sign_in",
            Self::Register => "register",
        }
    }
}

/// A challenge to store.
#[derive(Debug, Clone)]
pub struct NewChallenge {
    /// The ceremony's id, which the browser sends back.
    pub id: Uuid,
    /// What it is for.
    pub purpose: ChallengePurpose,
    /// Which surface asked.
    pub surface: ApiSurface,
    /// For registration: whose passkey it will be.
    pub user_id: Option<Uuid>,
    /// The random challenge.
    pub challenge: [u8; 32],
    /// How long it may be answered.
    pub lifetime: Duration,
}

/// Which challenge a ceremony's answer claims to answer.
#[derive(Debug, Clone, Copy)]
pub struct ChallengeKey {
    /// The ceremony's id.
    pub id: Uuid,
    /// What it must be for.
    pub purpose: ChallengePurpose,
    /// Which surface must have asked.
    pub surface: ApiSurface,
}

/// A challenge taken (deleted) to check an answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Challenge {
    /// For registration: whose passkey it will be.
    pub user_id: Option<Uuid>,
    /// The random challenge.
    pub challenge: Vec<u8>,
}

/// The options a browser needs to start a passkey ceremony.
#[derive(Debug, Clone)]
pub struct PasskeyOptions {
    /// The ceremony's id, sent back with the answer.
    pub ceremony_id: Uuid,
    /// The challenge.
    pub challenge: [u8; 32],
    /// The relying party ID.
    pub rp_id: String,
    /// How long the browser may wait for the user.
    pub timeout: Duration,
    /// Sign-in or registration details.
    pub kind: PasskeyOptionsKind,
}

/// What kind of ceremony the options start.
#[derive(Debug, Clone)]
pub enum PasskeyOptionsKind {
    /// Sign in with any of the user's passkeys (the browser offers them).
    SignIn,
    /// Make a new passkey.
    Register {
        /// The name browsers show for the site.
        rp_name: String,
        /// The user handle: the user's id.
        user_id: Uuid,
        /// The account name shown in the passkey manager (their email).
        user_name: String,
        /// Credential ids they already have, so an authenticator doesn't make a second.
        exclude: Vec<Vec<u8>>,
    },
}

/// A browser's answer to a sign-in ceremony, decoded.
#[derive(Debug, Clone)]
pub struct AuthenticationCredential {
    /// The credential id.
    pub raw_id: Vec<u8>,
    /// `clientDataJSON`.
    pub client_data_json: Vec<u8>,
    /// `authenticatorData`.
    pub authenticator_data: Vec<u8>,
    /// `signature`.
    pub signature: Vec<u8>,
    /// `userHandle`.
    pub user_handle: Option<Vec<u8>>,
}

/// A browser's answer to a registration ceremony, decoded.
#[derive(Debug, Clone)]
pub struct RegistrationCredential {
    /// `clientDataJSON`.
    pub client_data_json: Vec<u8>,
    /// `attestationObject`.
    pub attestation_object: Vec<u8>,
    /// `transports`, known values only.
    pub transports: Vec<String>,
}
