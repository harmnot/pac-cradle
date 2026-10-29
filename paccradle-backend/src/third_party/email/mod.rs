//! Outgoing email: sign-in links now, receipts and notices later.

mod email_sender_trait;
mod smtp;

pub use email_sender_trait::{EmailMessage, EmailSender};
pub use smtp::SmtpEmailSender;
