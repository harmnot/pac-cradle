use crate::error::AppError;
use crate::shared::future::BoxFuture;

/// A plain-text email to one person.
#[derive(Clone, PartialEq, Eq)]
pub struct EmailMessage {
    /// The recipient's address.
    pub to: String,
    /// The subject line.
    pub subject: String,
    /// The plain-text body.
    pub text: String,
}

// Bodies hold sign-in links, so they never reach a log line.
impl std::fmt::Debug for EmailMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EmailMessage")
            .field("subject", &self.subject)
            .finish_non_exhaustive()
    }
}

/// Sends email through a provider.
pub trait EmailSender: Send + Sync {
    /// Sends `message`. A provider that can't be reached or refuses is
    /// `SERVICE_UNAVAILABLE` (`meta.dependency = "email"`): the caller's job retries.
    fn send<'a>(&'a self, message: &'a EmailMessage) -> BoxFuture<'a, Result<(), AppError>>;
}
