//! Everything one design generates: the insert and, if the design has one, its outer box.

use serde::{Deserialize, Serialize};

use crate::design::Design;
use crate::error::GenerateError;
use crate::insert::Insert;
use crate::outer_box::OuterBox;

/// A generated packaging set (CLAUDE.md U4): the insert and the box round it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Packaging {
    pub insert: Insert,
    /// `None` when the design has no outer box.
    pub outer_box: Option<OuterBox>,
}

impl Design {
    /// Checks the design, then generates the insert and the outer box round it.
    pub fn generate(&self) -> Result<Packaging, GenerateError> {
        let insert = self.generate_insert()?;
        let outer_box = self
            .outer_box
            .as_ref()
            .map(|spec| spec.generate((self, &insert)))
            .transpose()?;
        Ok(Packaging { insert, outer_box })
    }
}
