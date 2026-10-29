use sqlx::PgConnection;
use time::OffsetDateTime;
use uuid::Uuid;

use super::PasskeyRepository;
use crate::error::{AppError, ErrorCode};
use crate::models::TextEnum;
use crate::models::passkey::{
    Challenge, ChallengeKey, NewChallenge, NewPasskey, Passkey, PasskeyUse,
};
use crate::shared::future::BoxFuture;

/// [`PasskeyRepository`] on Postgres.
#[derive(Default)]
pub struct PgPasskeyRepository;

impl PgPasskeyRepository {
    /// The repository.
    pub fn new() -> Self {
        Self
    }
}

struct PasskeyRow {
    id: Uuid,
    user_id: Uuid,
    credential_id: Vec<u8>,
    public_key: Vec<u8>,
    sign_count: i64,
    backup_eligible: bool,
    backed_up: bool,
    name: String,
    created_at: OffsetDateTime,
}

impl TryFrom<PasskeyRow> for Passkey {
    type Error = AppError;

    fn try_from(row: PasskeyRow) -> Result<Self, AppError> {
        Ok(Self {
            id: row.id,
            user_id: row.user_id,
            credential_id: row.credential_id,
            public_key: row.public_key,
            // The column's CHECK keeps it in u32's range.
            sign_count: u32::try_from(row.sign_count)
                .map_err(|error| AppError::new(ErrorCode::Internal).with_source(error))?,
            backup_eligible: row.backup_eligible,
            backed_up: row.backed_up,
            name: row.name,
            created_at: row.created_at,
        })
    }
}

impl PasskeyRepository for PgPasskeyRepository {
    fn insert_challenge<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        challenge: &'a NewChallenge,
    ) -> BoxFuture<'a, Result<(), AppError>> {
        Box::pin(async move {
            sqlx::query!(
                r#"
                INSERT INTO passkey_challenges (id, purpose, surface, user_id, challenge, expires_at)
                VALUES ($1, $2, $3, $4, $5, now() + make_interval(secs => $6))
                "#,
                challenge.id,
                challenge.purpose.as_str(),
                challenge.surface.as_str(),
                challenge.user_id,
                &challenge.challenge[..],
                challenge.lifetime.as_secs_f64(),
            )
            .execute(conn)
            .await?;
            Ok(())
        })
    }

    fn take_challenge<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        key: ChallengeKey,
    ) -> BoxFuture<'a, Result<Option<Challenge>, AppError>> {
        Box::pin(async move {
            let challenge = sqlx::query_as!(
                Challenge,
                r#"
                DELETE FROM passkey_challenges
                WHERE id = $1 AND purpose = $2 AND surface = $3 AND expires_at > now()
                RETURNING user_id, challenge
                "#,
                key.id,
                key.purpose.as_str(),
                key.surface.as_str(),
            )
            .fetch_optional(conn)
            .await?;
            Ok(challenge)
        })
    }

    fn delete_expired_challenges<'a>(
        &'a self,
        conn: &'a mut PgConnection,
    ) -> BoxFuture<'a, Result<u64, AppError>> {
        Box::pin(async move {
            let result = sqlx::query!("DELETE FROM passkey_challenges WHERE expires_at <= now()")
                .execute(conn)
                .await?;
            Ok(result.rows_affected())
        })
    }

    fn insert<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        passkey: &'a NewPasskey,
    ) -> BoxFuture<'a, Result<bool, AppError>> {
        Box::pin(async move {
            let credential = &passkey.credential;
            let result = sqlx::query!(
                r#"
                INSERT INTO passkeys (id, user_id, credential_id, public_key, algorithm, sign_count,
                                      transports, backup_eligible, backed_up, aaguid, name)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
                ON CONFLICT (credential_id) DO NOTHING
                "#,
                passkey.id,
                passkey.user_id,
                credential.credential_id,
                credential.public_key,
                credential.algorithm,
                i64::from(credential.sign_count),
                &passkey.transports,
                credential.backup_eligible,
                credential.backed_up,
                credential.aaguid.map(Uuid::from_bytes),
                passkey.name,
            )
            .execute(conn)
            .await?;
            Ok(result.rows_affected() == 1)
        })
    }

    fn find_by_credential_id<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        credential_id: &'a [u8],
    ) -> BoxFuture<'a, Result<Option<Passkey>, AppError>> {
        Box::pin(async move {
            let row = sqlx::query_as!(
                PasskeyRow,
                r#"
                SELECT id, user_id, credential_id, public_key, sign_count, backup_eligible,
                       backed_up, name, created_at
                FROM passkeys
                WHERE credential_id = $1
                "#,
                credential_id,
            )
            .fetch_optional(conn)
            .await?;
            row.map(Passkey::try_from).transpose()
        })
    }

    fn credential_ids<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        user_id: Uuid,
    ) -> BoxFuture<'a, Result<Vec<Vec<u8>>, AppError>> {
        Box::pin(async move {
            let ids = sqlx::query_scalar!(
                "SELECT credential_id FROM passkeys WHERE user_id = $1 ORDER BY created_at, id",
                user_id,
            )
            .fetch_all(conn)
            .await?;
            Ok(ids)
        })
    }

    fn record_use<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        (id, used): (Uuid, PasskeyUse),
    ) -> BoxFuture<'a, Result<(), AppError>> {
        Box::pin(async move {
            sqlx::query!(
                r#"
                UPDATE passkeys SET sign_count = $2, backed_up = $3, last_used_at = now()
                WHERE id = $1
                "#,
                id,
                i64::from(used.sign_count),
                used.backed_up,
            )
            .execute(conn)
            .await?;
            Ok(())
        })
    }
}
