//! Messages in the caller's language (`backend-architecture.md` §4.2). English only at launch
//! (DEC-11); a new language is a new file here and a new [`Locale`], nothing else.

mod en;
mod shared;

pub use shared::fill;

use axum::http::HeaderMap;
use axum::http::header::ACCEPT_LANGUAGE;

use crate::error::{ErrorCode, ErrorReport};
use crate::models::email_link::EmailLinkPurpose;

/// An email's text in one language, with `{name}` placeholders.
#[derive(Debug, Clone, Copy)]
pub struct EmailTemplate {
    /// The subject line.
    pub subject: &'static str,
    /// The plain-text body.
    pub body: &'static str,
    /// The word for hours, for `{valid_for}`.
    pub hours: &'static str,
    /// The word for minutes, for `{valid_for}`.
    pub minutes: &'static str,
}

/// A language the API can answer in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Locale {
    /// English, the default.
    #[default]
    En,
}

impl Locale {
    /// Every language, in the order offered.
    pub const ALL: [Self; 1] = [Self::En];

    /// The primary language subtag, e.g. `en`.
    pub const fn tag(self) -> &'static str {
        match self {
            Self::En => "en",
        }
    }

    /// The language with the primary subtag of `tag` (`en`, `en-GB`), or English.
    pub fn from_tag(tag: &str) -> Self {
        let primary = tag.split('-').next().unwrap_or(tag);
        Self::ALL
            .into_iter()
            .find(|locale| primary.eq_ignore_ascii_case(locale.tag()))
            .unwrap_or_default()
    }

    /// The caller's preferred language from `Accept-Language`, or English.
    pub fn from_headers(headers: &HeaderMap) -> Self {
        headers
            .get(ACCEPT_LANGUAGE)
            .and_then(|value| value.to_str().ok())
            .map_or_else(Self::default, shared::pick_locale)
    }

    /// The message template for an error code, with `{name}` placeholders.
    pub const fn error_template(self, code: ErrorCode) -> &'static str {
        match self {
            Self::En => en::error_message(code),
        }
    }

    /// The sign-in email for a link of `purpose`.
    pub const fn email_link(self, purpose: EmailLinkPurpose) -> EmailTemplate {
        match self {
            Self::En => en::email_link(purpose),
        }
    }

    /// The error's message with its meta filled in.
    pub fn error_message(self, report: &ErrorReport) -> String {
        fill(self.error_template(report.code), &report.meta)
    }
}
