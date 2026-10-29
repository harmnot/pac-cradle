//! A token bucket per key (a client address now, a user or route later), kept in memory.
//! One API process serves the site (CLAUDE.md §8.2), so no shared store is needed yet.

use std::collections::HashMap;
use std::hash::Hash;
use std::num::NonZeroU32;
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

/// Keys at which idle buckets are swept out, at least. Sweeps then run each time the map
/// doubles, so their cost spreads over many requests.
const MIN_SWEEP_AT: usize = 10_000;

/// Allows each key `per_minute` requests at once, refilled evenly over a minute.
pub struct RateLimiter<K> {
    capacity: f64,
    per_second: f64,
    state: Mutex<Buckets<K>>,
}

struct Buckets<K> {
    by_key: HashMap<K, Bucket>,
    sweep_at: usize,
}

#[derive(Clone, Copy)]
struct Bucket {
    tokens: f64,
    updated: Instant,
}

impl<K: Hash + Eq> RateLimiter<K> {
    /// A limiter allowing `per_minute` requests per key.
    pub fn new(per_minute: NonZeroU32) -> Self {
        let capacity = f64::from(per_minute.get());
        Self {
            capacity,
            per_second: capacity / 60.0,
            state: Mutex::new(Buckets {
                by_key: HashMap::new(),
                sweep_at: MIN_SWEEP_AT,
            }),
        }
    }

    /// Spends one request for `key` at `now`. When the key has none left, returns how long
    /// until it has one again.
    pub fn check(&self, key: K, now: Instant) -> Result<(), Duration> {
        // A panic while holding the lock leaves the counts usable, so carry on with them.
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        if state.by_key.len() >= state.sweep_at {
            self.sweep(&mut state, now);
        }
        let bucket = state.by_key.entry(key).or_insert(Bucket {
            tokens: self.capacity,
            updated: now,
        });
        bucket.tokens = self.refilled(*bucket, now);
        bucket.updated = now;
        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            return Ok(());
        }
        Err(Duration::from_secs_f64(
            (1.0 - bucket.tokens) / self.per_second,
        ))
    }

    /// How many keys are being tracked.
    pub fn tracked_keys(&self) -> usize {
        self.state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .by_key
            .len()
    }

    /// Drops the buckets that have filled up again: those keys start fresh anyway.
    fn sweep(&self, state: &mut Buckets<K>, now: Instant) {
        state
            .by_key
            .retain(|_, bucket| self.refilled(*bucket, now) < self.capacity);
        state.sweep_at = (state.by_key.len() * 2).max(MIN_SWEEP_AT);
    }

    fn refilled(&self, bucket: Bucket, now: Instant) -> f64 {
        let elapsed = now.saturating_duration_since(bucket.updated).as_secs_f64();
        (bucket.tokens + elapsed * self.per_second).min(self.capacity)
    }
}
