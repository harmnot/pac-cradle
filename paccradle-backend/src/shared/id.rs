//! IDs: UUID v7 everywhere (backend rules). They sort by creation time, which keeps
//! database indexes compact and log lines in order.

use uuid::Uuid;

/// A new UUID v7.
pub fn new_id() -> Uuid {
    Uuid::now_v7()
}
