//! The `email_links` table.

mod email_link_repository;
mod email_link_repository_trait;

pub use email_link_repository::PgEmailLinkRepository;
pub use email_link_repository_trait::EmailLinkRepository;
