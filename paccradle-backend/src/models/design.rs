//! Designs a user saves (the `designs` table, CLAUDE.md §6, §8.4): the editor's
//! `insert_core::Design`, with a name and a version.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use super::TextEnum;
use super::page::{PageRequest, SortOrder};

/// The longest design name, in characters (`designs_name_length`).
pub const MAX_DESIGN_NAME_CHARS: usize = 200;

/// The insert styles a design can have, as `insert_core` names them in the spec.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InsertStyle {
    /// Slotted dividers.
    PartitionGrid,
    /// A flat card with product-shaped holes.
    CutoutCard,
    /// A raised deck on folded legs.
    CradlePlatform,
}

impl TextEnum for InsertStyle {
    const ALL: &'static [Self] = &[Self::PartitionGrid, Self::CutoutCard, Self::CradlePlatform];

    fn as_str(self) -> &'static str {
        match self {
            Self::PartitionGrid => "partition_grid",
            Self::CutoutCard => "cutout_card",
            Self::CradlePlatform => "cradle_platform",
        }
    }
}

/// A design to add.
#[derive(Debug, Clone)]
pub struct NewDesign {
    /// The id it will have.
    pub id: Uuid,
    /// Whose it is.
    pub user_id: Uuid,
    /// Its name, trimmed; may be empty.
    pub name: String,
    /// What the editor made, already checked.
    pub spec: insert_core::Design,
}

/// A saved design.
#[derive(Debug, Clone, PartialEq)]
pub struct SavedDesign {
    /// The design's id.
    pub id: Uuid,
    /// Whose it is.
    pub user_id: Uuid,
    /// Its name; may be empty.
    pub name: String,
    /// The products (copies of their outlines), insert, material and box.
    pub spec: insert_core::Design,
    /// 1 when made, one more on every saved change. An export records the version it was
    /// made from (M1-D7).
    pub version: i32,
    /// When it was made.
    pub created_at: OffsetDateTime,
    /// When it last changed.
    pub updated_at: OffsetDateTime,
}

/// A change to a saved design, made from the version the editor had.
#[derive(Debug, Clone)]
pub struct DesignChange {
    /// The version the change was made from; another one means someone saved in between.
    pub from_version: i32,
    /// A new name, if it changes.
    pub name: Option<String>,
    /// A new spec, already checked, if it changes.
    pub spec: Option<insert_core::Design>,
}

/// A design in the "My designs" list: no outlines, just what the list shows.
#[derive(Debug, Clone, PartialEq)]
pub struct DesignSummary {
    /// The design's id.
    pub id: Uuid,
    /// Its name; may be empty.
    pub name: String,
    /// Its version.
    pub version: i32,
    /// Its insert style.
    pub insert_style: InsertStyle,
    /// How many product types it holds.
    pub product_types: i32,
    /// How many products it holds in all.
    pub product_count: i64,
    /// Whether it has an outer box.
    pub has_box: bool,
    /// When it was made.
    pub created_at: OffsetDateTime,
    /// When it last changed.
    pub updated_at: OffsetDateTime,
}

/// What "My designs" is sorted by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DesignSort {
    /// Last changed (the default).
    #[default]
    UpdatedAt,
    /// Made.
    CreatedAt,
    /// Name, ignoring letter case.
    Name,
}

impl DesignSort {
    /// The name the SQL compares.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UpdatedAt => "updated_at",
            Self::CreatedAt => "created_at",
            Self::Name => "name",
        }
    }
}

/// One page of a user's designs, filtered and sorted.
#[derive(Debug, Clone)]
pub struct DesignListQuery {
    /// Whose designs.
    pub user_id: Uuid,
    /// Which page.
    pub page: PageRequest,
    /// Only names containing this, ignoring letter case.
    pub search: Option<String>,
    /// Only this insert style.
    pub style: Option<InsertStyle>,
    /// Sorted by.
    pub sort: DesignSort,
    /// Which way.
    pub order: SortOrder,
}
