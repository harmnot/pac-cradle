use std::error::Error as _;
use std::sync::Arc;

use super::JobHandler;
use crate::error::{AppError, ErrorCode};
use crate::models::email_link::{SendEmailLink, SendEmailLinkPayload};
use crate::models::job::JobError;
use crate::service::identity::IdentityService;

/// Sends sign-up and sign-in links (`send_email_link`), through the identity service.
pub struct SendEmailLinkHandler {
    identity: Arc<dyn IdentityService>,
}

impl SendEmailLinkHandler {
    /// A handler sending through `identity`.
    pub fn new(identity: Arc<dyn IdentityService>) -> Self {
        Self { identity }
    }
}

impl JobHandler for SendEmailLinkHandler {
    type Kind = SendEmailLink;

    async fn run(&self, payload: SendEmailLinkPayload) -> Result<(), JobError> {
        self.identity
            .deliver_email_link(payload.link_id)
            .await
            .map_err(job_error)
    }
}

/// A provider or database outage is retried; anything else (an address the provider can't
/// take) never will work.
fn job_error(error: AppError) -> JobError {
    let reason = match error.source() {
        Some(source) => format!("{error}: {source}"),
        None => error.to_string(),
    };
    match error.code() {
        ErrorCode::ServiceUnavailable | ErrorCode::Internal => JobError::Retry(reason),
        _ => JobError::GiveUp(reason),
    }
}
