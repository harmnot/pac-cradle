use crate::error::ErrorCode;
use crate::models::email_link::EmailLinkPurpose;

use super::EmailTemplate;

/// English error messages. `{name}` takes the error's meta value of that name.
pub(super) const fn error_message(code: ErrorCode) -> &'static str {
    match code {
        ErrorCode::InvalidRequest => "The request isn't valid.",
        ErrorCode::AuthRequired => "Please sign in to continue.",
        ErrorCode::Forbidden => "You don't have access to this.",
        ErrorCode::RouteNotFound => "There is nothing at this address.",
        ErrorCode::MethodNotAllowed => "This address doesn't accept that method.",
        ErrorCode::PayloadTooLarge => "The request is too large.",
        ErrorCode::UnsupportedMediaType => "This type of content isn't accepted here.",
        ErrorCode::RateLimited => "Too many requests. Please try again in {retry_after_seconds} s.",
        ErrorCode::Internal => "Something went wrong on our side. Please try again.",
        ErrorCode::ServiceUnavailable => {
            "The service is unavailable right now. Please try again in a few seconds."
        }
        ErrorCode::RequestTimeout => "The request took too long. Please try again.",
        ErrorCode::AuthInvalidCredentials => "The email or password isn't right.",
        ErrorCode::AuthSessionExpired => "Your session has ended. Please sign in again.",
        ErrorCode::AuthLinkInvalid => {
            "This link has expired or was already used. Ask for a new one."
        }
        ErrorCode::AuthAccountRequired => "Confirm your email to do this.",
        ErrorCode::AuthReauthRequired => {
            "Enter your current password, or sign in again with a passkey or an email link."
        }
        ErrorCode::AuthPasswordRejected => {
            "Choose a password of {min_length} to {max_length} characters."
        }
        ErrorCode::AuthPasskeyRejected => "That passkey didn't work. Please try again.",
        ErrorCode::AuthPasskeyAlreadyRegistered => "This passkey is already saved.",
    }
}

/// English sign-in emails. `{link}` is the link; `{valid_for}` how long it works.
pub(super) const fn email_link(purpose: EmailLinkPurpose) -> EmailTemplate {
    match purpose {
        EmailLinkPurpose::SignUp => EmailTemplate {
            subject: "Confirm your email for Paccradle",
            body: "Hi,\n\n\
                   Confirm your email address to finish creating your Paccradle account:\n\n\
                   {link}\n\n\
                   The link works once and expires in {valid_for}. If you didn't ask for it, \
                   ignore this email: no account is made without it.\n\n\
                   Paccradle\n",
            hours: "hours",
            minutes: "minutes",
        },
        EmailLinkPurpose::SignIn => EmailTemplate {
            subject: "Your Paccradle sign-in link",
            body: "Hi,\n\n\
                   Use this link to sign in to Paccradle:\n\n\
                   {link}\n\n\
                   It works once and expires in {valid_for}. Once signed in, you can set a new \
                   password.\n\n\
                   If you didn't ask for it, ignore this email: your account stays as it is.\n\n\
                   Paccradle\n",
            hours: "hours",
            minutes: "minutes",
        },
    }
}
