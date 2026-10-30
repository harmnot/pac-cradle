//! The sheet under the product: A4 or US Letter (CLAUDE.md §3 step 2).

use serde::{Deserialize, Serialize};

use super::TextEnum;

/// `users.paper_default` (the paper the capture screen starts with) and `captures.paper`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Paper {
    /// 210 × 297 mm.
    A4,
    /// 8.5 × 11 in.
    Letter,
}

impl TextEnum for Paper {
    const ALL: &'static [Self] = &[Self::A4, Self::Letter];

    fn as_str(self) -> &'static str {
        match self {
            Self::A4 => "a4",
            Self::Letter => "letter",
        }
    }
}
