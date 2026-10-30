//! Design bodies (M1-D6): a design to save, a change to one, the design as stored, and a
//! page of "My designs".

use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use super::common::page_request;
use crate::error::{AppError, ErrorCode};
use crate::models::design::{
    DesignChange, DesignSort, DesignSummary, InsertStyle, MAX_DESIGN_NAME_CHARS, SavedDesign,
};
use crate::models::page::{PageRequest, SortOrder};

/// `POST /designs`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateDesignRequest {
    /// Shown in "My designs"; at most 200 characters, may be left out.
    #[serde(default)]
    pub name: Option<String>,
    /// The editor's design: `insert_core::Design` (material, products with their outlines
    /// copied from the capture, insert, outer box).
    pub spec: Value,
}

impl CreateDesignRequest {
    /// The name (trimmed; empty when left out) and the spec, read.
    pub fn parts(self) -> Result<(String, insert_core::Design), AppError> {
        let name = short_text(("name", self.name))?.unwrap_or_default();
        Ok((name, read_spec(self.spec)?))
    }
}

/// `PATCH /designs/{design_id}`: the version the editor changed, and what changes.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateDesignRequest {
    /// The version the change was made from (the `version` last read or saved).
    pub version: i32,
    /// A new name.
    #[serde(default)]
    pub name: Option<String>,
    /// A new spec, whole.
    #[serde(default)]
    pub spec: Option<Value>,
}

impl UpdateDesignRequest {
    /// The change. Neither a name nor a spec is `INVALID_REQUEST`.
    pub fn change(self) -> Result<DesignChange, AppError> {
        if self.name.is_none() && self.spec.is_none() {
            return Err(AppError::new(ErrorCode::InvalidRequest)
                .with_meta("field", "name")
                .with_meta("expected", "a name, a spec, or both"));
        }
        Ok(DesignChange {
            from_version: self.version,
            name: short_text(("name", self.name))?,
            spec: self.spec.map(read_spec).transpose()?,
        })
    }
}

/// `GET /designs`: which page, filtered and sorted.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesignListParams {
    /// From 1; default 1.
    pub page: Option<u32>,
    /// 1 to 100; default 20.
    pub page_size: Option<u32>,
    /// Only names containing this, ignoring letter case.
    pub search: Option<String>,
    /// Only this insert style.
    pub style: Option<InsertStyle>,
    /// `updated_at` (default), `created_at` or `name`.
    pub sort: Option<DesignSort>,
    /// `asc` or `desc`; default `asc` by name, `desc` by date.
    pub order: Option<SortOrder>,
}

/// What a list query asks for, checked.
#[derive(Debug, Clone)]
pub struct DesignListFilters {
    /// Which page.
    pub page: PageRequest,
    /// The search text, trimmed; none when empty.
    pub search: Option<String>,
    /// The insert style.
    pub style: Option<InsertStyle>,
    /// Sorted by.
    pub sort: DesignSort,
    /// Which way.
    pub order: SortOrder,
}

impl DesignListParams {
    /// The filters, with their defaults.
    pub fn filters(self) -> Result<DesignListFilters, AppError> {
        let sort = self.sort.unwrap_or_default();
        let default_order = match sort {
            DesignSort::Name => SortOrder::Asc,
            DesignSort::UpdatedAt | DesignSort::CreatedAt => SortOrder::Desc,
        };
        Ok(DesignListFilters {
            page: page_request((self.page, self.page_size))?,
            search: short_text(("search", self.search))?.filter(|search| !search.is_empty()),
            style: self.style,
            sort,
            order: self.order.unwrap_or(default_order),
        })
    }
}

/// A saved design: `POST` (201), `GET` and `PATCH /designs/{design_id}`.
#[derive(Debug, Serialize)]
pub struct DesignResponse {
    /// The design's id.
    pub design_id: Uuid,
    /// Its name; may be empty.
    pub name: String,
    /// Send it back with the next `PATCH`.
    pub version: i32,
    /// The design as saved.
    pub spec: insert_core::Design,
    /// When it was made.
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    /// When it last changed.
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

impl From<SavedDesign> for DesignResponse {
    fn from(design: SavedDesign) -> Self {
        Self {
            design_id: design.id,
            name: design.name,
            version: design.version,
            spec: design.spec,
            created_at: design.created_at,
            updated_at: design.updated_at,
        }
    }
}

/// A design in "My designs": what the list shows, without outlines.
#[derive(Debug, Serialize)]
pub struct DesignSummaryResponse {
    /// The design's id.
    pub design_id: Uuid,
    /// Its name; may be empty.
    pub name: String,
    /// Its version.
    pub version: i32,
    /// `partition_grid`, `cutout_card` or `cradle_platform`.
    pub insert_style: InsertStyle,
    /// Product types it holds.
    pub product_types: i32,
    /// Products it holds in all (every type's quantity).
    pub product_count: i64,
    /// Whether it has an outer box.
    pub has_box: bool,
    /// When it was made.
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    /// When it last changed.
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

impl From<DesignSummary> for DesignSummaryResponse {
    fn from(design: DesignSummary) -> Self {
        Self {
            design_id: design.id,
            name: design.name,
            version: design.version,
            insert_style: design.insert_style,
            product_types: design.product_types,
            product_count: design.product_count,
            has_box: design.has_box,
            created_at: design.created_at,
            updated_at: design.updated_at,
        }
    }
}

/// A name (or a search for one) in `field`: control characters dropped, trimmed, at most
/// 200 characters.
fn short_text((field, name): (&'static str, Option<String>)) -> Result<Option<String>, AppError> {
    let Some(name) = name else {
        return Ok(None);
    };
    let name: String = name.chars().filter(|c| !c.is_control()).collect();
    let name = name.trim();
    if name.chars().count() > MAX_DESIGN_NAME_CHARS {
        return Err(AppError::new(ErrorCode::InvalidRequest)
            .with_meta("field", field)
            .with_meta(
                "allowed",
                format!("at most {MAX_DESIGN_NAME_CHARS} characters"),
            ));
    }
    Ok(Some(name.to_owned()))
}

/// The spec as `insert_core` reads it. A wrong shape is `DESIGN_INVALID` with serde's words;
/// the rules are checked by the design service.
fn read_spec(spec: Value) -> Result<insert_core::Design, AppError> {
    serde_json::from_value(spec).map_err(|error| {
        AppError::new(ErrorCode::DesignInvalid).with_meta("detail", error.to_string())
    })
}
