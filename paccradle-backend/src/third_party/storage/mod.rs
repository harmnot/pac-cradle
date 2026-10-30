//! Object storage for photos (M1-D4) and, later, export files (M1-D7): private objects,
//! reached by others only through short-lived signed URLs.

mod cloudflare_r2;
mod memory;
mod storage_trait;

pub use cloudflare_r2::CloudflareR2;
pub use memory::MemoryStorage;
pub use storage_trait::{NewObject, ObjectStorage};
