use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::passkey::{
    Challenge, ChallengeKey, NewChallenge, NewPasskey, Passkey, PasskeyUse,
};
use crate::shared::future::BoxFuture;

/// Reads and writes passkeys and their ceremonies' challenges, on the connection it is given.
pub trait PasskeyRepository: Send + Sync {
    /// Stores a ceremony's challenge.
    fn insert_challenge<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        challenge: &'a NewChallenge,
    ) -> BoxFuture<'a, Result<(), AppError>>;

    /// Deletes and returns the unexpired challenge `key` names, so it can be answered once.
    fn take_challenge<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        key: ChallengeKey,
    ) -> BoxFuture<'a, Result<Option<Challenge>, AppError>>;

    /// Deletes expired challenges. Returns how many.
    fn delete_expired_challenges<'a>(
        &'a self,
        conn: &'a mut PgConnection,
    ) -> BoxFuture<'a, Result<u64, AppError>>;

    /// Adds a passkey. `false` when its credential id is already stored.
    fn insert<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        passkey: &'a NewPasskey,
    ) -> BoxFuture<'a, Result<bool, AppError>>;

    /// The passkey with this credential id.
    fn find_by_credential_id<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        credential_id: &'a [u8],
    ) -> BoxFuture<'a, Result<Option<Passkey>, AppError>>;

    /// The credential ids of the user's passkeys, oldest first.
    fn credential_ids<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        user_id: Uuid,
    ) -> BoxFuture<'a, Result<Vec<Vec<u8>>, AppError>>;

    /// Records a sign-in with the passkey.
    fn record_use<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        use_of: (Uuid, PasskeyUse),
    ) -> BoxFuture<'a, Result<(), AppError>>;
}
