//! The paccradle API (CLAUDE.md §8): one Axum service with two surfaces, each with its own
//! routes, URL prefix and semver contract (`backend-architecture.md` §4.8):
//!
//! - `/api/client/v1/…` for the web app's users,
//! - `/api/admin/v1/…` for the back office,
//!
//! plus the unversioned probes `/healthz` and `/readyz` for the deploy scripts. Background
//! jobs run in the same process ([`jobs::Worker`]), on a Postgres job table.

pub mod config;
pub mod error;
pub mod handlers;
pub mod jobs;
pub mod locales;
pub mod middleware;
pub mod models;
pub mod repository;
pub mod schemas;
pub mod service;
pub mod shared;
pub mod third_party;

mod app_state;

pub use app_state::{AppState, Providers};

use axum::Router;

/// The whole HTTP application: every route, wrapped in the shared middleware.
pub fn app(state: AppState) -> Router {
    let config = state.config.clone();
    middleware::stack(handlers::router(state), &config)
}
