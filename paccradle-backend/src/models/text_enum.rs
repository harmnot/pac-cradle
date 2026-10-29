use crate::error::{AppError, ErrorCode};

/// An enum stored as lower-case snake_case text in a checked column (`'a4'`, `'passkey'`).
/// Implement [`TextEnum::ALL`] and [`TextEnum::as_str`]; reading back comes for free.
pub trait TextEnum: Sized + Copy + 'static {
    /// Every value.
    const ALL: &'static [Self];

    /// The value as stored.
    fn as_str(self) -> &'static str;

    /// The value stored as `text`. Text no value has means the column's CHECK and this enum
    /// disagree, a bug: 500.
    fn from_text(text: &str) -> Result<Self, AppError> {
        Self::ALL
            .iter()
            .copied()
            .find(|value| value.as_str() == text)
            .ok_or_else(|| {
                AppError::new(ErrorCode::Internal).with_source(UnknownValue(text.to_owned()))
            })
    }
}

/// A stored value no enum variant has.
#[derive(Debug, thiserror::Error)]
#[error("unknown stored value {0:?}")]
struct UnknownValue(String);
