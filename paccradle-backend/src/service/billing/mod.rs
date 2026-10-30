//! Billing: what a user may export, and the checkout that unlocks a design (CLAUDE.md §9.3,
//! M1-D7). Paddle's webhooks, purchases, refunds and the portal join with M1-F.

mod billing_service;
mod billing_service_trait;

pub use billing_service::{BillingProviders, DefaultBillingService};
pub use billing_service_trait::{BillingService, ExportEntitlement};
