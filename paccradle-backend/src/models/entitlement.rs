//! What a user may export (`design_unlocks`, `credit_ledger`, `subscriptions`; CLAUDE.md
//! §9.3, §9.4). Paddle keeps the money; these say what is paid for.

use std::time::Duration;

use serde::Serialize;
use time::OffsetDateTime;
use uuid::Uuid;

use super::TextEnum;

/// How long after an unlock its design may still be changed and exported again (§9.4).
/// Downloads of exports already made never expire.
pub const EDIT_WINDOW: Duration = Duration::from_secs(30 * 24 * 60 * 60);

/// `design_unlocks.source`: what paid for a design's production files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UnlockSource {
    /// A single-export checkout (M1-F4).
    Purchase,
    /// One export credit.
    Credit,
    /// A Pro subscription.
    Pro,
}

impl TextEnum for UnlockSource {
    const ALL: &'static [Self] = &[Self::Purchase, Self::Credit, Self::Pro];

    fn as_str(self) -> &'static str {
        match self {
            Self::Purchase => "purchase",
            Self::Credit => "credit",
            Self::Pro => "pro",
        }
    }
}

/// A design's unlock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesignUnlock {
    /// The design.
    pub design_id: Uuid,
    /// Who it is unlocked for.
    pub user_id: Uuid,
    /// What paid.
    pub source: UnlockSource,
    /// Versions saved until then are covered.
    pub edits_until: OffsetDateTime,
}

impl DesignUnlock {
    /// Whether it covers the design version saved at `saved_at`: production files of it can
    /// be made without paying again (§9.4: edited for 30 days).
    pub fn covers(&self, saved_at: OffsetDateTime) -> bool {
        saved_at <= self.edits_until
    }
}

/// An unlock to make or renew, covering [`EDIT_WINDOW`] from now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NewUnlock {
    /// The id it gets if the design has no unlock yet.
    pub id: Uuid,
    /// The design.
    pub design_id: Uuid,
    /// Who it is for.
    pub user_id: Uuid,
    /// What paid: a credit or Pro here; purchases come with M1-F4.
    pub source: UnlockSource,
}

/// `exports.paid_with`: how an export's paid formats were covered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PaidWith {
    /// Only free formats (the PNG preview, the test print).
    Free,
    /// The design was already unlocked, and this version was saved within its edit window.
    Unlocked,
    /// The user's Pro subscription (the design is now unlocked).
    Pro,
    /// One export credit, spent now (the design is now unlocked).
    Credit,
}

impl TextEnum for PaidWith {
    const ALL: &'static [Self] = &[Self::Free, Self::Unlocked, Self::Pro, Self::Credit];

    fn as_str(self) -> &'static str {
        match self {
            Self::Free => "free",
            Self::Unlocked => "unlocked",
            Self::Pro => "pro",
            Self::Credit => "credit",
        }
    }
}

/// One export credit spent on a design.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreditSpend {
    /// The ledger row's id.
    pub id: Uuid,
    /// Whose credit.
    pub user_id: Uuid,
    /// The design it unlocks.
    pub design_id: Uuid,
}

/// What a user's exports are paid with, as `/me` shows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Plan {
    /// No subscription: free files, plus credits or single purchases.
    Free,
    /// Pro: every export (§10).
    Pro,
    /// Studio (v2): Pro for 3 seats.
    Studio,
}

/// `subscriptions.plan`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionPlan {
    /// Pro, $12 a month.
    ProMonthly,
    /// Pro, $139 a year.
    ProYearly,
    /// Studio, $49 a month (v2).
    StudioMonthly,
}

impl TextEnum for SubscriptionPlan {
    const ALL: &'static [Self] = &[Self::ProMonthly, Self::ProYearly, Self::StudioMonthly];

    fn as_str(self) -> &'static str {
        match self {
            Self::ProMonthly => "pro_monthly",
            Self::ProYearly => "pro_yearly",
            Self::StudioMonthly => "studio_monthly",
        }
    }
}

/// `subscriptions.status`, as Paddle last said it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionStatus {
    /// In a free trial.
    Trialing,
    /// Paid up.
    Active,
    /// A payment failed; Paddle is still trying.
    PastDue,
    /// Paused by the customer.
    Paused,
    /// Ended.
    Canceled,
}

impl SubscriptionStatus {
    /// Whether it gives the plan's exports: trialing, active, or past due while Paddle
    /// collects (the same rule as `EntitlementRepository::pro_active`).
    pub const fn grants_plan(self) -> bool {
        matches!(self, Self::Trialing | Self::Active | Self::PastDue)
    }
}

impl TextEnum for SubscriptionStatus {
    const ALL: &'static [Self] = &[
        Self::Trialing,
        Self::Active,
        Self::PastDue,
        Self::Paused,
        Self::Canceled,
    ];

    fn as_str(self) -> &'static str {
        match self {
            Self::Trialing => "trialing",
            Self::Active => "active",
            Self::PastDue => "past_due",
            Self::Paused => "paused",
            Self::Canceled => "canceled",
        }
    }
}

/// A user's subscription, as Paddle last told us.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Subscription {
    /// Which plan.
    pub plan: SubscriptionPlan,
    /// Its state.
    pub status: SubscriptionStatus,
    /// When the paid period ends (renews or stops).
    pub current_period_end: Option<OffsetDateTime>,
}

/// What `/me` says about a user's billing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BillingSummary {
    /// Free, or the plan a live subscription gives.
    pub plan: Plan,
    /// Export credits left.
    pub credits: i64,
    /// The subscription that matters: a live one, else the latest.
    pub subscription: Option<Subscription>,
    /// Designs unlocked for them.
    pub unlocked_designs: i64,
}
