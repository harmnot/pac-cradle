use std::fmt;

use axum::http::StatusCode;

/// A stable error code. Callers branch on it and translate it; it never changes once
/// released (a new meaning gets a new code, a minor version bump).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCode {
    /// 400: the request can't be understood (bad JSON, a missing or wrong field).
    InvalidRequest,
    /// 401: the caller must sign in first.
    AuthRequired,
    /// 403: the caller is signed in but may not do this.
    Forbidden,
    /// 404: no route at this path.
    RouteNotFound,
    /// 405: the route exists but not for this method. The `allow` header lists the methods.
    MethodNotAllowed,
    /// 413: the body is over the route's limit.
    PayloadTooLarge,
    /// 415: the body's content type isn't accepted.
    UnsupportedMediaType,
    /// 429: too many requests; `retry-after` says when to try again.
    RateLimited,
    /// 500: a bug or an unexpected failure on our side.
    Internal,
    /// 503: a dependency (the database, a provider) is down.
    ServiceUnavailable,
    /// 503: the request ran past `HTTP_REQUEST_TIMEOUT_SECONDS`.
    RequestTimeout,
    /// 401: the email and password don't match an account (or the account has no password).
    AuthInvalidCredentials,
    /// 401: the refresh cookie is missing, unknown, expired or revoked: sign in again.
    AuthSessionExpired,
    /// 400: the email link is unknown, used or expired.
    AuthLinkInvalid,
    /// 403: a guest asked for something that needs an account (a confirmed email).
    AuthAccountRequired,
    /// 403: changing the password needs the current one, or a sign-in by email link or passkey
    /// in the last `{fresh_minutes}` minutes.
    AuthReauthRequired,
    /// 400: the new password is shorter than `{min_length}` or longer than `{max_length}`
    /// characters.
    AuthPasswordRejected,
    /// 400: the passkey couldn't be checked (wrong origin, expired ceremony, bad signature…).
    AuthPasskeyRejected,
    /// 409: this passkey is already registered.
    AuthPasskeyAlreadyRegistered,
}

impl ErrorCode {
    /// Every code, so tests can check each has a message in every language.
    pub const ALL: [Self; 19] = [
        Self::InvalidRequest,
        Self::AuthRequired,
        Self::Forbidden,
        Self::RouteNotFound,
        Self::MethodNotAllowed,
        Self::PayloadTooLarge,
        Self::UnsupportedMediaType,
        Self::RateLimited,
        Self::Internal,
        Self::ServiceUnavailable,
        Self::RequestTimeout,
        Self::AuthInvalidCredentials,
        Self::AuthSessionExpired,
        Self::AuthLinkInvalid,
        Self::AuthAccountRequired,
        Self::AuthReauthRequired,
        Self::AuthPasswordRejected,
        Self::AuthPasskeyRejected,
        Self::AuthPasskeyAlreadyRegistered,
    ];

    /// The code as callers see it, e.g. `ROUTE_NOT_FOUND`.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidRequest => "INVALID_REQUEST",
            Self::AuthRequired => "AUTH_REQUIRED",
            Self::Forbidden => "FORBIDDEN",
            Self::RouteNotFound => "ROUTE_NOT_FOUND",
            Self::MethodNotAllowed => "METHOD_NOT_ALLOWED",
            Self::PayloadTooLarge => "PAYLOAD_TOO_LARGE",
            Self::UnsupportedMediaType => "UNSUPPORTED_MEDIA_TYPE",
            Self::RateLimited => "RATE_LIMITED",
            Self::Internal => "INTERNAL_ERROR",
            Self::ServiceUnavailable => "SERVICE_UNAVAILABLE",
            Self::RequestTimeout => "REQUEST_TIMEOUT",
            Self::AuthInvalidCredentials => "AUTH_INVALID_CREDENTIALS",
            Self::AuthSessionExpired => "AUTH_SESSION_EXPIRED",
            Self::AuthLinkInvalid => "AUTH_LINK_INVALID",
            Self::AuthAccountRequired => "AUTH_ACCOUNT_REQUIRED",
            Self::AuthReauthRequired => "AUTH_REAUTH_REQUIRED",
            Self::AuthPasswordRejected => "AUTH_PASSWORD_REJECTED",
            Self::AuthPasskeyRejected => "AUTH_PASSKEY_REJECTED",
            Self::AuthPasskeyAlreadyRegistered => "AUTH_PASSKEY_ALREADY_REGISTERED",
        }
    }

    /// The HTTP status the code is sent with.
    pub const fn status(self) -> StatusCode {
        match self {
            Self::InvalidRequest
            | Self::AuthLinkInvalid
            | Self::AuthPasswordRejected
            | Self::AuthPasskeyRejected => StatusCode::BAD_REQUEST,
            Self::AuthRequired | Self::AuthInvalidCredentials | Self::AuthSessionExpired => {
                StatusCode::UNAUTHORIZED
            }
            Self::Forbidden | Self::AuthAccountRequired | Self::AuthReauthRequired => {
                StatusCode::FORBIDDEN
            }
            Self::AuthPasskeyAlreadyRegistered => StatusCode::CONFLICT,
            Self::RouteNotFound => StatusCode::NOT_FOUND,
            Self::MethodNotAllowed => StatusCode::METHOD_NOT_ALLOWED,
            Self::PayloadTooLarge => StatusCode::PAYLOAD_TOO_LARGE,
            Self::UnsupportedMediaType => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            Self::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            Self::Internal => StatusCode::INTERNAL_SERVER_ERROR,
            Self::ServiceUnavailable | Self::RequestTimeout => StatusCode::SERVICE_UNAVAILABLE,
        }
    }

    /// The closest code for an error response that framework code made without an
    /// [`super::AppError`] (an extractor's rejection, a panic). Its status is kept.
    pub const fn from_status(status: StatusCode) -> Self {
        match status.as_u16() {
            401 => Self::AuthRequired,
            403 => Self::Forbidden,
            404 => Self::RouteNotFound,
            405 => Self::MethodNotAllowed,
            413 => Self::PayloadTooLarge,
            415 => Self::UnsupportedMediaType,
            429 => Self::RateLimited,
            503 => Self::ServiceUnavailable,
            400..=499 => Self::InvalidRequest,
            _ => Self::Internal,
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
