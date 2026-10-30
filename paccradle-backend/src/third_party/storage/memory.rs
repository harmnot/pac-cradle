use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use super::{NewObject, ObjectStorage};
use crate::error::{AppError, ErrorCode};
use crate::shared::future::BoxFuture;

/// [`ObjectStorage`] in this process's memory: for tests, and for development without
/// `STORAGE_*` settings. Objects are lost at restart, and its signed URLs (`memory:///…`)
/// can't be opened. Tests can make it go down ([`MemoryStorage::set_available`]).
pub struct MemoryStorage {
    objects: Mutex<HashMap<String, (Vec<u8>, &'static str)>>,
    available: AtomicBool,
}

impl Default for MemoryStorage {
    fn default() -> Self {
        Self {
            objects: Mutex::new(HashMap::new()),
            available: AtomicBool::new(true),
        }
    }
}

impl MemoryStorage {
    /// An empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// The object at `key` and its media type.
    pub fn object(&self, key: &str) -> Option<(Vec<u8>, &'static str)> {
        self.objects().get(key).cloned()
    }

    /// Every key, sorted.
    pub fn keys(&self) -> Vec<String> {
        let mut keys: Vec<String> = self.objects().keys().cloned().collect();
        keys.sort_unstable();
        keys
    }

    /// While `false`, every call fails as an unreachable store would.
    pub fn set_available(&self, available: bool) {
        self.available.store(available, Ordering::SeqCst);
    }

    fn objects(&self) -> std::sync::MutexGuard<'_, HashMap<String, (Vec<u8>, &'static str)>> {
        // A panic while holding the lock leaves the map usable.
        self.objects.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn reachable(&self) -> Result<(), AppError> {
        if self.available.load(Ordering::SeqCst) {
            return Ok(());
        }
        Err(AppError::new(ErrorCode::ServiceUnavailable).with_meta("dependency", "storage"))
    }
}

impl ObjectStorage for MemoryStorage {
    fn put(&self, object: NewObject) -> BoxFuture<'_, Result<(), AppError>> {
        Box::pin(async move {
            self.reachable()?;
            self.objects()
                .insert(object.key, (object.bytes, object.content_type));
            Ok(())
        })
    }

    fn get<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<Option<Vec<u8>>, AppError>> {
        Box::pin(async move {
            self.reachable()?;
            Ok(self.objects().get(key).map(|(bytes, _)| bytes.clone()))
        })
    }

    fn delete<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<(), AppError>> {
        Box::pin(async move {
            self.reachable()?;
            self.objects().remove(key);
            Ok(())
        })
    }

    fn signed_url(&self, (key, valid_for): (&str, Duration)) -> Result<String, AppError> {
        Ok(format!(
            "memory:///{key}?expires_in={}",
            valid_for.as_secs()
        ))
    }
}
