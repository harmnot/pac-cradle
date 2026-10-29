//! Futures returned by trait methods. `async fn` in a trait can't be called through
//! `dyn Trait` (Rust 1.99), and services, repositories and providers live in `AppState` as
//! `Arc<dyn …>` so tests can swap them, so their methods return a [`BoxFuture`] made with
//! `Box::pin(async move { … })`.

use std::future::Future;
use std::pin::Pin;

/// A boxed, sendable future borrowing for `'a`.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
