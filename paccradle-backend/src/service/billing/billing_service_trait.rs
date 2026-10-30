use sqlx::PgConnection;

use crate::error::AppError;
use crate::models::design::SavedDesign;
use crate::models::entitlement::PaidWith;
use crate::models::session::AuthUser;
use crate::shared::future::BoxFuture;

/// A design's production files, asked for by its owner.
#[derive(Debug, Clone, Copy)]
pub struct ExportEntitlement<'a> {
    /// The owner (an account, not a guest).
    pub user: &'a AuthUser,
    /// The design as it will be exported.
    pub design: &'a SavedDesign,
}

/// What users have paid for (CLAUDE.md §9.3, §9.4).
pub trait BillingService: Send + Sync {
    /// Covers a design's production files, in `conn` (the caller's transaction, so the credit,
    /// the unlock and the export commit together), with the user's row locked:
    ///
    /// 1. already unlocked, and this version saved within the unlock's 30-day edit window:
    ///    `Unlocked`, nothing spent;
    /// 2. Pro (active, trialing or past due): the design is unlocked for 30 days, `Pro`;
    /// 3. a credit: one is spent and the design unlocked for 30 days, `Credit`;
    /// 4. otherwise `EXPORT_PAYMENT_REQUIRED` (402), whose `meta.checkout` opens Paddle for a
    ///    single export of this design and `meta.offers` lists the other price ids.
    fn entitle_export<'a>(
        &'a self,
        request: (&'a mut PgConnection, ExportEntitlement<'a>),
    ) -> BoxFuture<'a, Result<PaidWith, AppError>>;
}
