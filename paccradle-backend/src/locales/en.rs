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
        ErrorCode::CaptureFileTooLarge => {
            "The photo is too large. Send one of at most {max_mb} MB, as a JPEG."
        }
        ErrorCode::CaptureUnsupportedImage => {
            "This file isn't a JPEG, PNG or WebP photo. Send the photo as a JPEG."
        }
        ErrorCode::CaptureUnreadableImage => {
            "The photo can't be read: it may be damaged or cut short. Please take it again."
        }
        ErrorCode::CaptureImageTooLarge => {
            "The photo has too many pixels ({width} × {height}). Send one of at most \
             {max_megapixels} megapixels."
        }
        ErrorCode::CaptureImageTooSmall => {
            "The photo is too small to measure ({width} × {height} px). Its long side needs at \
             least {min_long_side} px."
        }
        ErrorCode::CaptureInvalidInput => "The setting {field} must be {allowed}.",
        ErrorCode::CapturePaperNotFound => {
            "We couldn't find the paper. Put it on a darker surface and keep all 4 corners in \
             the photo."
        }
        ErrorCode::CaptureProductNotFound => {
            "We couldn't find the product on the paper. Tap it, or put it in the middle, at \
             least 2 cm from the edges, and take the photo again."
        }
        ErrorCode::CaptureTraceNotFound => {
            "We couldn't find a closed pen outline. Trace all the way round the product, then \
             take it away before the photo."
        }
        ErrorCode::CaptureNothingAtTap => {
            "Nothing was found where you tapped. Tap the middle of the product, or use Trace \
             mode if it's close to the paper's colour."
        }
        ErrorCode::CaptureOutlineCrossesItself => {
            "The outline crosses itself. Move the points so the line goes round the product once."
        }
        ErrorCode::CaptureProcessingFailed => {
            "Something went wrong measuring the photo. Tap the product to try again."
        }
        ErrorCode::CaptureNotFound => "There is no such capture.",
        ErrorCode::CapturePhotoDeleted => {
            "This photo was deleted after 30 days. Take a new one to measure again."
        }
        ErrorCode::CaptureNotReady => {
            "There's no outline to edit yet. Wait for the measurement, or tap the product."
        }
        ErrorCode::DesignNotFound => "There is no such design.",
        ErrorCode::DesignInvalid => "This design can't be saved: {detail}.",
        ErrorCode::DesignVersionConflict => {
            "This design was changed somewhere else (version {current_version}). Reload it to \
             see the latest."
        }
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
