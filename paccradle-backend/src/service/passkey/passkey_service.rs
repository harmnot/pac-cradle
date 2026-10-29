use std::sync::Arc;
use std::time::Duration;

use super::{PasskeyRegistration, PasskeyService, PasskeySignIn};
use crate::config::AppConfig;
use crate::error::{AppError, ErrorCode};
use crate::models::passkey::{
    ChallengeKey, ChallengePurpose, NewChallenge, NewPasskey, Passkey, PasskeyOptions,
    PasskeyOptionsKind, PasskeyUse,
};
use crate::models::session::{AuthUser, SignInMethod, SignedIn};
use crate::models::user::AccountRole;
use crate::repository::database::Database;
use crate::repository::passkey::PasskeyRepository;
use crate::repository::user::UserRepository;
use crate::service::session::{SessionService, StartSession};
use crate::shared::api_surface::ApiSurface;
use crate::shared::crypto::random_bytes;
use crate::shared::future::BoxFuture;
use crate::shared::id::new_id;
use crate::shared::webauthn::{
    self, Authentication, Expected, Registration, StoredCredential, WebauthnError,
};

/// How long a ceremony may take, from options to answer.
pub const CEREMONY_TTL: Duration = Duration::from_secs(5 * 60);

/// What [`DefaultPasskeyService`] works with.
pub struct PasskeyProviders {
    /// Transactions.
    pub db: Arc<dyn Database>,
    /// `passkeys` and `passkey_challenges`.
    pub passkeys: Arc<dyn PasskeyRepository>,
    /// `users`.
    pub users: Arc<dyn UserRepository>,
    /// Sessions and tokens.
    pub sessions: Arc<dyn SessionService>,
}

/// [`PasskeyService`] on Postgres, for `WEBAUTHN_RP_ID` and the surfaces' web origins.
pub struct DefaultPasskeyService {
    config: Arc<AppConfig>,
    parts: PasskeyProviders,
}

impl DefaultPasskeyService {
    /// A service with `config` (RP ID and name, web origins) on `parts`.
    pub fn new(config: Arc<AppConfig>, parts: PasskeyProviders) -> Self {
        Self { config, parts }
    }

    /// The web page origin a surface's ceremonies must come from.
    fn origin(&self, surface: ApiSurface) -> Result<&str, AppError> {
        let origin = match surface {
            ApiSurface::Client => Some(&self.config.web_origin),
            ApiSurface::Admin => self.config.admin_web_origin.as_ref(),
        };
        origin
            .and_then(|origin| origin.to_str().ok())
            .ok_or_else(|| rejected(WebauthnError("no web origin for this surface")))
    }

    async fn new_ceremony(
        &self,
        (purpose, surface, user_id): (ChallengePurpose, ApiSurface, Option<uuid::Uuid>),
    ) -> Result<NewChallenge, AppError> {
        let challenge = NewChallenge {
            id: new_id(),
            purpose,
            surface,
            user_id,
            challenge: random_bytes()?,
            lifetime: CEREMONY_TTL,
        };
        let mut conn = self.parts.db.connection().await?;
        let passkeys = &self.parts.passkeys;
        passkeys.delete_expired_challenges(&mut conn).await?;
        passkeys.insert_challenge(&mut conn, &challenge).await?;
        Ok(challenge)
    }

    fn options(&self, challenge: &NewChallenge, kind: PasskeyOptionsKind) -> PasskeyOptions {
        PasskeyOptions {
            ceremony_id: challenge.id,
            challenge: challenge.challenge,
            rp_id: self.config.auth.rp_id.clone(),
            timeout: CEREMONY_TTL,
            kind,
        }
    }
}

impl PasskeyService for DefaultPasskeyService {
    fn sign_in_options(
        &self,
        surface: ApiSurface,
    ) -> BoxFuture<'_, Result<PasskeyOptions, AppError>> {
        Box::pin(async move {
            self.origin(surface)?;
            let challenge = self
                .new_ceremony((ChallengePurpose::SignIn, surface, None))
                .await?;
            Ok(self.options(&challenge, PasskeyOptionsKind::SignIn))
        })
    }

    fn sign_in<'a>(
        &'a self,
        sign_in: PasskeySignIn<'a>,
    ) -> BoxFuture<'a, Result<SignedIn, AppError>> {
        Box::pin(async move {
            let surface = sign_in.surface;
            let origin = self.origin(surface)?;
            let credential = &sign_in.credential;
            let key = ChallengeKey {
                id: sign_in.ceremony_id,
                purpose: ChallengePurpose::SignIn,
                surface,
            };
            // Taken on its own connection, so it is gone even when the answer is refused.
            let mut conn = self.parts.db.connection().await?;
            let challenge = self
                .parts
                .passkeys
                .take_challenge(&mut conn, key)
                .await?
                .ok_or_else(|| rejected(WebauthnError("unknown or expired ceremony")))?;
            let passkey = self
                .parts
                .passkeys
                .find_by_credential_id(&mut conn, &credential.raw_id)
                .await?
                .ok_or_else(|| rejected(WebauthnError("unknown passkey")))?;
            let user = self
                .parts
                .users
                .find(&mut conn, passkey.user_id)
                .await?
                .ok_or_else(|| rejected(WebauthnError("passkey without an owner")))?;
            drop(conn);
            if surface == ApiSurface::Admin && user.account_role() != AccountRole::Admin {
                return Err(rejected(WebauthnError("not an admin's passkey")));
            }

            let expected = Expected {
                challenge: &challenge.challenge,
                origin,
                rp_id: &self.config.auth.rp_id,
            };
            let stored = StoredCredential {
                public_key: &passkey.public_key,
                sign_count: passkey.sign_count,
                backup_eligible: passkey.backup_eligible,
                user_handle: user.id.as_bytes(),
            };
            let response = Authentication {
                client_data_json: &credential.client_data_json,
                authenticator_data: &credential.authenticator_data,
                signature: &credential.signature,
                user_handle: credential.user_handle.as_deref(),
            };
            let assertion = webauthn::verify_authentication(&expected, (&stored, &response))
                .map_err(rejected)?;

            let mut tx = self.parts.db.begin().await?;
            let used = PasskeyUse {
                sign_count: assertion.sign_count,
                backed_up: assertion.backed_up,
            };
            self.parts
                .passkeys
                .record_use(&mut tx, (passkey.id, used))
                .await?;
            let start = StartSession {
                user_id: user.id,
                surface,
                method: SignInMethod::Passkey,
                replacing: sign_in.replacing,
            };
            let signed_in = self.parts.sessions.start(&mut tx, start).await?;
            tx.commit().await?;
            Ok(signed_in)
        })
    }

    fn registration_options<'a>(
        &'a self,
        auth: &'a AuthUser,
    ) -> BoxFuture<'a, Result<PasskeyOptions, AppError>> {
        Box::pin(async move {
            let mut conn = self.parts.db.connection().await?;
            let user = self
                .parts
                .users
                .find(&mut conn, auth.user_id)
                .await?
                .ok_or_else(|| AppError::new(ErrorCode::AuthRequired))?;
            let Some(email) = user.email.clone() else {
                return Err(AppError::new(ErrorCode::AuthAccountRequired));
            };
            let exclude = self
                .parts
                .passkeys
                .credential_ids(&mut conn, user.id)
                .await?;
            drop(conn);
            let challenge = self
                .new_ceremony((ChallengePurpose::Register, auth.surface, Some(user.id)))
                .await?;
            let kind = PasskeyOptionsKind::Register {
                rp_name: self.config.auth.rp_name.clone(),
                user_id: user.id,
                user_name: email,
                exclude,
            };
            Ok(self.options(&challenge, kind))
        })
    }

    fn register<'a>(
        &'a self,
        (auth, registration): (&'a AuthUser, PasskeyRegistration),
    ) -> BoxFuture<'a, Result<Passkey, AppError>> {
        Box::pin(async move {
            let origin = self.origin(auth.surface)?;
            let key = ChallengeKey {
                id: registration.ceremony_id,
                purpose: ChallengePurpose::Register,
                surface: auth.surface,
            };
            let mut conn = self.parts.db.connection().await?;
            let challenge = self
                .parts
                .passkeys
                .take_challenge(&mut conn, key)
                .await?
                .filter(|challenge| challenge.user_id == Some(auth.user_id))
                .ok_or_else(|| rejected(WebauthnError("unknown or expired ceremony")))?;
            let expected = Expected {
                challenge: &challenge.challenge,
                origin,
                rp_id: &self.config.auth.rp_id,
            };
            let credential = &registration.credential;
            let response = Registration {
                client_data_json: &credential.client_data_json,
                attestation_object: &credential.attestation_object,
            };
            let checked = webauthn::verify_registration(&expected, &response).map_err(rejected)?;
            let passkey = NewPasskey {
                id: new_id(),
                user_id: auth.user_id,
                credential: checked,
                transports: credential.transports.clone(),
                name: registration.name,
            };
            if !self.parts.passkeys.insert(&mut conn, &passkey).await? {
                return Err(AppError::new(ErrorCode::AuthPasskeyAlreadyRegistered));
            }
            tracing::info!(user = %auth.user_id, passkey = %passkey.id, "passkey added");
            self.parts
                .passkeys
                .find_by_credential_id(&mut conn, &passkey.credential.credential_id)
                .await?
                .ok_or_else(|| AppError::new(ErrorCode::Internal))
        })
    }
}

/// `AUTH_PASSKEY_REJECTED`, keeping the reason for the logs.
fn rejected(reason: WebauthnError) -> AppError {
    AppError::new(ErrorCode::AuthPasskeyRejected).with_source(reason)
}
