use std::time::Duration;

use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::entitlement::{CreditSpend, DesignUnlock, NewUnlock};
use crate::shared::future::BoxFuture;

/// Reads and writes what a user may export. Every method runs on the connection it is given:
/// the billing service runs them in the caller's transaction, with the user's row locked.
pub trait EntitlementRepository: Send + Sync {
    /// Locks the user's row until the transaction ends, so one user's credits are spent one
    /// at a time (Postgres alone wouldn't stop a balance going below zero).
    fn lock_account<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        user_id: Uuid,
    ) -> BoxFuture<'a, Result<(), AppError>>;

    /// The design's unlock, if it has one.
    fn unlock<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        design_id: Uuid,
    ) -> BoxFuture<'a, Result<Option<DesignUnlock>, AppError>>;

    /// Unlocks the design for `window` from now, or renews its unlock with the new source.
    fn save_unlock<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        unlock: (&'a NewUnlock, Duration),
    ) -> BoxFuture<'a, Result<DesignUnlock, AppError>>;

    /// The user's export credits: the sum of their ledger.
    fn credit_balance<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        user_id: Uuid,
    ) -> BoxFuture<'a, Result<i64, AppError>>;

    /// Adds a `spend` row of −1 for the design.
    fn spend_credit<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        spend: &'a CreditSpend,
    ) -> BoxFuture<'a, Result<(), AppError>>;

    /// Whether the user has a Pro subscription Paddle calls active, trialing or past due
    /// (Paddle is still collecting).
    fn pro_active<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        user_id: Uuid,
    ) -> BoxFuture<'a, Result<bool, AppError>>;
}
