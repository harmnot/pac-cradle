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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
