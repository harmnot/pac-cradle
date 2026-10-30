//! Billing bodies (M1-D8): the `billing` block of `/me`, and a design's unlock.

use serde::Serialize;
use time::OffsetDateTime;

use crate::models::entitlement::{
    BillingSummary, DesignUnlock, Plan, SubscriptionPlan, SubscriptionStatus, UnlockSource,
};

/// What a user's exports are paid with.
#[derive(Debug, Serialize)]
pub struct BillingResponse {
    /// `free`, `pro` or `studio`: what a live subscription gives.
    pub plan: Plan,
    /// Export credits left (never expire).
    pub credits: i64,
    /// The subscription that matters (a live one, else the latest), if any.
    pub subscription: Option<SubscriptionResponse>,
    /// How many designs are unlocked (each design says so itself: `unlock`).
    pub unlocked_designs: i64,
}

/// A subscription as Paddle last told us.
#[derive(Debug, Serialize)]
pub struct SubscriptionResponse {
    /// `pro_monthly`, `pro_yearly` or `studio_monthly`.
    pub plan: SubscriptionPlan,
    /// `trialing`, `active`, `past_due` (still gives the plan), `paused` or `canceled`.
    pub status: SubscriptionStatus,
    /// When the paid period ends.
    #[serde(with = "time::serde::rfc3339::option")]
    pub current_period_end: Option<OffsetDateTime>,
}

impl From<BillingSummary> for BillingResponse {
    fn from(summary: BillingSummary) -> Self {
        Self {
            plan: summary.plan,
            credits: summary.credits,
            subscription: summary
                .subscription
                .map(|subscription| SubscriptionResponse {
                    plan: subscription.plan,
                    status: subscription.status,
                    current_period_end: subscription.current_period_end,
                }),
            unlocked_designs: summary.unlocked_designs,
        }
    }
}

/// A design's unlock: its production files are paid for.
#[derive(Debug, Serialize)]
pub struct UnlockResponse {
    /// `purchase`, `credit` or `pro`.
    pub source: UnlockSource,
    /// Versions saved until then export without paying again.
    #[serde(with = "time::serde::rfc3339")]
    pub edits_until: OffsetDateTime,
    /// Whether the design as it is now is covered: its production files export at no cost.
    pub covers_current_version: bool,
}

impl UnlockResponse {
    /// The unlock, judged against the design's version saved at `saved_at`.
    pub fn new((unlock, saved_at): (&DesignUnlock, OffsetDateTime)) -> Self {
        Self {
            source: unlock.source,
            edits_until: unlock.edits_until,
            covers_current_version: unlock.covers(saved_at),
        }
    }
}
