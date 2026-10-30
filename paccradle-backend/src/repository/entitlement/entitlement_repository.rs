use std::time::Duration;

use sqlx::PgConnection;
use time::OffsetDateTime;
use uuid::Uuid;

use super::EntitlementRepository;
use crate::error::AppError;
use crate::models::TextEnum;
use crate::models::entitlement::{
    CreditSpend, DesignUnlock, NewUnlock, Subscription, SubscriptionPlan, SubscriptionStatus,
    UnlockSource,
};
use crate::shared::future::BoxFuture;

/// [`EntitlementRepository`] on Postgres.
#[derive(Default)]
pub struct PgEntitlementRepository;

impl PgEntitlementRepository {
    /// The repository.
    pub fn new() -> Self {
        Self
    }
}

struct UnlockRow {
    design_id: Uuid,
    user_id: Uuid,
    source: String,
    edits_until: OffsetDateTime,
}

impl TryFrom<UnlockRow> for DesignUnlock {
    type Error = AppError;

    fn try_from(row: UnlockRow) -> Result<Self, AppError> {
        Ok(Self {
            design_id: row.design_id,
            user_id: row.user_id,
            source: UnlockSource::from_text(&row.source)?,
            edits_until: row.edits_until,
        })
    }
}

struct SubscriptionRow {
    plan: String,
    status: String,
    current_period_end: Option<OffsetDateTime>,
}

impl TryFrom<SubscriptionRow> for Subscription {
    type Error = AppError;

    fn try_from(row: SubscriptionRow) -> Result<Self, AppError> {
        Ok(Self {
            plan: SubscriptionPlan::from_text(&row.plan)?,
            status: SubscriptionStatus::from_text(&row.status)?,
            current_period_end: row.current_period_end,
        })
    }
}

impl EntitlementRepository for PgEntitlementRepository {
    fn lock_account<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        user_id: Uuid,
    ) -> BoxFuture<'a, Result<(), AppError>> {
        Box::pin(async move {
            sqlx::query_scalar!("SELECT id FROM users WHERE id = $1 FOR UPDATE", user_id)
                .fetch_optional(conn)
                .await?;
            Ok(())
        })
    }

    fn unlock<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        design_id: Uuid,
    ) -> BoxFuture<'a, Result<Option<DesignUnlock>, AppError>> {
        Box::pin(async move {
            let row = sqlx::query_as!(
                UnlockRow,
                r#"
                SELECT design_id, user_id, source, edits_until
                FROM design_unlocks WHERE design_id = $1
                "#,
                design_id,
            )
            .fetch_optional(conn)
            .await?;
            row.map(DesignUnlock::try_from).transpose()
        })
    }

    fn save_unlock<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        (unlock, window): (&'a NewUnlock, Duration),
    ) -> BoxFuture<'a, Result<DesignUnlock, AppError>> {
        Box::pin(async move {
            let row = sqlx::query_as!(
                UnlockRow,
                r#"
                INSERT INTO design_unlocks (id, design_id, user_id, source, edits_until)
                VALUES ($1, $2, $3, $4, now() + make_interval(secs => $5))
                ON CONFLICT (design_id) DO UPDATE
                SET user_id = EXCLUDED.user_id,
                    source = EXCLUDED.source,
                    paddle_transaction_id = NULL,
                    unlocked_at = now(),
                    edits_until = EXCLUDED.edits_until
                RETURNING design_id, user_id, source, edits_until
                "#,
                unlock.id,
                unlock.design_id,
                unlock.user_id,
                unlock.source.as_str(),
                window.as_secs_f64(),
            )
            .fetch_one(conn)
            .await?;
            DesignUnlock::try_from(row)
        })
    }

    fn credit_balance<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        user_id: Uuid,
    ) -> BoxFuture<'a, Result<i64, AppError>> {
        Box::pin(async move {
            let balance = sqlx::query_scalar!(
                r#"SELECT coalesce(sum(delta), 0)::bigint AS "balance!" FROM credit_ledger WHERE user_id = $1"#,
                user_id,
            )
            .fetch_one(conn)
            .await?;
            Ok(balance)
        })
    }

    fn spend_credit<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        spend: &'a CreditSpend,
    ) -> BoxFuture<'a, Result<(), AppError>> {
        Box::pin(async move {
            sqlx::query!(
                r#"
                INSERT INTO credit_ledger (id, user_id, delta, reason, design_id)
                VALUES ($1, $2, -1, 'spend', $3)
                "#,
                spend.id,
                spend.user_id,
                spend.design_id,
            )
            .execute(conn)
            .await?;
            Ok(())
        })
    }

    fn pro_active<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        user_id: Uuid,
    ) -> BoxFuture<'a, Result<bool, AppError>> {
        Box::pin(async move {
            let active = sqlx::query_scalar!(
                r#"
                SELECT EXISTS (
                    SELECT 1 FROM subscriptions
                    WHERE user_id = $1 AND status IN ('active', 'trialing', 'past_due')
                ) AS "active!"
                "#,
                user_id,
            )
            .fetch_one(conn)
            .await?;
            Ok(active)
        })
    }

    fn subscription<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        user_id: Uuid,
    ) -> BoxFuture<'a, Result<Option<Subscription>, AppError>> {
        Box::pin(async move {
            let row = sqlx::query_as!(
                SubscriptionRow,
                r#"
                SELECT plan, status, current_period_end
                FROM subscriptions
                WHERE user_id = $1
                ORDER BY status IN ('active', 'trialing', 'past_due') DESC, paddle_updated_at DESC
                LIMIT 1
                "#,
                user_id,
            )
            .fetch_optional(conn)
            .await?;
            row.map(Subscription::try_from).transpose()
        })
    }

    fn unlock_count<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        user_id: Uuid,
    ) -> BoxFuture<'a, Result<i64, AppError>> {
        Box::pin(async move {
            let count = sqlx::query_scalar!(
                r#"SELECT count(*) AS "count!" FROM design_unlocks WHERE user_id = $1"#,
                user_id,
            )
            .fetch_one(conn)
            .await?;
            Ok(count)
        })
    }

    fn unlocks<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        (user_id, design_ids): (Uuid, &'a [Uuid]),
    ) -> BoxFuture<'a, Result<Vec<DesignUnlock>, AppError>> {
        Box::pin(async move {
            let rows = sqlx::query_as!(
                UnlockRow,
                r#"
                SELECT design_id, user_id, source, edits_until
                FROM design_unlocks
                WHERE user_id = $1 AND design_id = ANY($2)
                "#,
                user_id,
                design_ids,
            )
            .fetch_all(conn)
            .await?;
            rows.into_iter().map(DesignUnlock::try_from).collect()
        })
    }
}
