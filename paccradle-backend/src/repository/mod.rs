//! Database access (`backend-architecture.md` §4.4): SQL only, called only from services.
//! Each table's repository arrives with the task that first uses the table.

pub mod audit;
pub mod database;
pub mod email_link;
pub mod health;
pub mod job;
pub mod passkey;
pub mod session;
pub mod user;
