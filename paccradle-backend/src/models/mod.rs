//! Domain types for the database tables (`backend-architecture.md` §4.3). Each table gets its
//! model with the task that first reads or writes it (users in M1-D3, captures in D4, …).

pub mod audit;
pub mod capture;
pub mod design;
pub mod email_link;
pub mod job;
pub mod page;
pub mod paper;
pub mod passkey;
pub mod session;
mod text_enum;
pub mod user;

pub use text_enum::TextEnum;
