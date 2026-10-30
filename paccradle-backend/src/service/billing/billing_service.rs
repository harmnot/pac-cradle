use std::sync::Arc;

use serde_json::json;
use sqlx::PgConnection;

use super::{BillingService, ExportEntitlement};
use crate::config::PaddlePrices;
use crate::error::{AppError, ErrorCode};
use crate::models::entitlement::{CreditSpend, EDIT_WINDOW, NewUnlock, PaidWith, UnlockSource};
use crate::repository::entitlement::EntitlementRepository;
use crate::repository::user::UserRepository;
use crate::shared::future::BoxFuture;
use crate::shared::id::new_id;

/// What [`DefaultBillingService`] works with.
pub struct BillingProviders {
    /// Unlocks, credits and subscriptions.
    pub entitlements: Arc<dyn EntitlementRepository>,
    /// `users` (the checkout's email).
    pub users: Arc<dyn UserRepository>,
    /// Paddle's price ids.
    pub prices: PaddlePrices,
}

/// [`BillingService`] on Postgres.
pub struct DefaultBillingService {
    parts: BillingProviders,
}

impl DefaultBillingService {
    /// A service on `parts`.
    pub fn new(parts: BillingProviders) -> Self {
        Self { parts }
    }

    /// Unlocks the design for the edit window, paid by `source`.
    async fn unlock(
        &self,
        (conn, request, source): (&mut PgConnection, &ExportEntitlement<'_>, UnlockSource),
    ) -> Result<(), AppError> {
        let unlock = NewUnlock {
            id: new_id(),
            design_id: request.design.id,
            user_id: request.user.user_id,
            source,
        };
        self.parts
            .entitlements
            .save_unlock(conn, (&unlock, EDIT_WINDOW))
            .await?;
        Ok(())
    }

    /// `EXPORT_PAYMENT_REQUIRED` with what Paddle.js needs to sell this design's export.
    async fn payment_required(
        &self,
        (conn, request): (&mut PgConnection, &ExportEntitlement<'_>),
    ) -> Result<AppError, AppError> {
        let user = self.parts.users.find(conn, request.user.user_id).await?;
        let prices = &self.parts.prices;
        let checkout = json!({
            "price_id": prices.single_export,
            "custom_data": {
                "user_id": request.user.user_id,
                "design_id": request.design.id,
            },
            "customer_email": user.and_then(|user| user.email),
        });
        let offers = json!({
            "single_export": prices.single_export,
            "export_pack": prices.export_pack,
            "pro_monthly": prices.pro_monthly,
            "pro_yearly": prices.pro_yearly,
        });
        Ok(AppError::new(ErrorCode::ExportPaymentRequired)
            .with_meta("checkout", checkout)
            .with_meta("offers", offers))
    }
}

impl BillingService for DefaultBillingService {
    fn entitle_export<'a>(
        &'a self,
        (conn, request): (&'a mut PgConnection, ExportEntitlement<'a>),
    ) -> BoxFuture<'a, Result<PaidWith, AppError>> {
        Box::pin(async move {
            let user_id = request.user.user_id;
            let entitlements = &self.parts.entitlements;
            entitlements.lock_account(conn, user_id).await?;
            let unlock = entitlements.unlock(conn, request.design.id).await?;
            if unlock.is_some_and(|unlock| request.design.updated_at <= unlock.edits_until) {
                return Ok(PaidWith::Unlocked);
            }
            if entitlements.pro_active(conn, user_id).await? {
                self.unlock((conn, &request, UnlockSource::Pro)).await?;
                return Ok(PaidWith::Pro);
            }
            if entitlements.credit_balance(conn, user_id).await? > 0 {
                let spend = CreditSpend {
                    id: new_id(),
                    user_id,
                    design_id: request.design.id,
                };
                entitlements.spend_credit(conn, &spend).await?;
                self.unlock((conn, &request, UnlockSource::Credit)).await?;
                return Ok(PaidWith::Credit);
            }
            Err(self.payment_required((conn, &request)).await?)
        })
    }
}
